package remoteshell

import (
	"github.com/json-iterator/go"
    "fmt"
    "net/http"
    "os"
    "os/exec"
    _ "os/signal"
    "syscall"
    _ "time"

    "github.com/creack/pty"
    "github.com/gorilla/websocket"
)

var json = jsoniter.ConfigCompatibleWithStandardLibrary

type CommandArgs struct {
    Command string   `json:"command"`
    Args    []string `json:"args,omitempty"`
}

type ClientMessage struct {
    Op    string `json:"op"`
    Stdin string `json:"stdin,omitempty"`
}

type ServerMessage struct {
    Op     string `json:"op"`
    Stdout string `json:"stdout,omitempty"`
}

var upgrader = websocket.Upgrader{
    CheckOrigin: func(r *http.Request) bool {
        return true
    },
}

func WsHandler(w http.ResponseWriter, r *http.Request) {
    conn, err := upgrader.Upgrade(w, r, nil)
    if err != nil {
        fmt.Println("Upgrade error:", err)
        return
    }
    defer conn.Close()

    query := r.URL.Query()
    args := query.Get("args")
    if args == "" {
        args = `{"command":"/bin/sh"}`
    }

    var commandArgs CommandArgs
    if err := json.Unmarshal([]byte(args), &commandArgs); err != nil {
        fmt.Println("JSON unmarshal error:", err)
        return
    }
	// 使用绝对路径
	commandPath, err := exec.LookPath(commandArgs.Command)
	if err != nil {
		fmt.Printf("Command %s not found\n", commandArgs.Command)
		return
	}
    // Create a PTY
    master, slave, err := openPty()
    if err != nil {
        fmt.Println("PTY error:", err)
        return
    }
    defer master.Close()
    defer slave.Close()
// Fork a child process

    cmd := exec.Command(commandPath, commandArgs.Args...)
    cmd.Stdout = slave
    cmd.Stderr = slave
    cmd.Stdin = slave
    cmd.SysProcAttr = &syscall.SysProcAttr{Setsid: true}
	// 传递环境变量，但移除 TERM
	env := os.Environ()
	/*
	for i, v := range env {
		if len(v) >= 5 && v[:5] == "TERM=" {
			env = append(env[:i], env[i+1:]...)
			break
		}
	}*/
	cmd.Env = env
	//cmd.Env = os.Environ() // 传递环境变量

    if err := cmd.Start(); err != nil {
        fmt.Println("ForkExec error:", err)
        return
    }

    go func() {
        buf := make([]byte, 1024)
        for {
            n, err := master.Read(buf)
            if err != nil {
                fmt.Println("Read error:", err)
                break
            }
            if n > 0 {
		    serverMessage := ServerMessage{
		        Op:     "out",
		        Stdout: string(buf[:n]),
		    }
		    message, _ := json.Marshal(serverMessage)
		    //if err := conn.WriteMessage(websocket.BinaryMessage, buf[:n]); err != nil {
		    if err := conn.WriteMessage(websocket.TextMessage, message); err != nil {
                    fmt.Println("WriteMessage error:", err)
                    break
                }
            }
        }
    }()

    for {
        _, message, err := conn.ReadMessage()
        if err != nil {
            fmt.Println("ReadMessage error:", err)
            break
        }

        var clientMessage ClientMessage
        if err := json.Unmarshal(message, &clientMessage); err != nil {
            fmt.Println("JSON unmarshal error:", err)
            continue
        }

        switch clientMessage.Op {
        case "ping":
            serverMessage := ServerMessage{
                Op: "pong",
            }
            response, _ := json.Marshal(serverMessage)
            conn.WriteMessage(websocket.TextMessage, response)
        case "input":
            if _, err := master.Write([]byte(clientMessage.Stdin)); err != nil {
                fmt.Println("Write error:", err)
            }
        default:
            fmt.Println("Unknown operation:", clientMessage.Op)
        }
    }

    // Wait for the child process to exit
	cmd.Wait();
	/*/
    // Fork a child process
	//fmt.Println("commandArgs.command=", commandArgs.Command, commandArgs)
    pid, err := syscall.ForkExec(commandPath, commandArgs.Args, &syscall.ProcAttr{
        Files: []uintptr{slave.Fd(), slave.Fd(), slave.Fd()},
        Sys:   &syscall.SysProcAttr{Setsid: true},
    })
    if err != nil {
        fmt.Println("ForkExec error:", err)
        return
    }

    go func() {
        buf := make([]byte, 1024)
        for {
            n, err := master.Read(buf)
            if err != nil {
                fmt.Println("Read error:", err)
                break
            }
            if n > 0 {
		    //serverMessage := ServerMessage{
		    //    Op:     "out",
		    //    Stdout: string(buf[:n]),
		    //}
		    //message, _ := json.Marshal(serverMessage)
		    //if err := conn.WriteMessage(websocket.TextMessage, message); err != nil {
		    if err := conn.WriteMessage(websocket.BinaryMessage, buf[:n]); err != nil {
                    fmt.Println("WriteMessage error:", err)
                    break
                }
            }
        }
    }()

    for {
        _, message, err := conn.ReadMessage()
        if err != nil {
            fmt.Println("ReadMessage error:", err)
            break
        }

        var clientMessage ClientMessage
        if err := json.Unmarshal(message, &clientMessage); err != nil {
            fmt.Println("JSON unmarshal error:", err)
            continue
        }

        switch clientMessage.Op {
        case "ping":
            serverMessage := ServerMessage{
                Op: "pong",
            }
            response, _ := json.Marshal(serverMessage)
            conn.WriteMessage(websocket.TextMessage, response)
        case "input":
            if _, err := master.Write([]byte(clientMessage.Stdin)); err != nil {
                fmt.Println("Write error:", err)
            }
        default:
            fmt.Println("Unknown operation:", clientMessage.Op)
        }
    }

    // Send SIGTERM to the child process
    syscall.Kill(pid, syscall.SIGTERM)

    // Wait for the child process to exit
    syscall.Wait4(pid, nil, 0, nil)
	*/
}

func openPty() (*os.File, *os.File, error) {
    master, slave, err := pty.Open()
    if err != nil {
        return nil, nil, err
    }
    return master, slave, nil
}
