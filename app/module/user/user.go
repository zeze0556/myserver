package user

import (
	"github.com/json-iterator/go"
	_ "io/ioutil"
	"bufio"
	"os"
	 "os/exec"
	"time"
	"fmt"
	 "strings"
	_ "github.com/tidwall/gjson"
	"github.com/google/uuid"
	"net/http"
)

var json = jsoniter.ConfigCompatibleWithStandardLibrary

type HttpUser struct {
	api_token string
	username string
}

type UserInfoRepo struct {
}

var client map[string]*HttpUser = make(map[string]*HttpUser);

type LoginRequest struct {
	Username string `json:"username"`
	Password string `json:"password"`
}

type LoginResponse struct {
	Ret  int    `json:"ret"`
	//Message string `json:"message"`
}

func NewUser(api_token string, username string) *HttpUser {
	return &HttpUser{
		api_token: api_token,
		username: username,
	}
}

func HandleUserinfo(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Invalid request method", http.StatusMethodNotAllowed)
		return
	}
	// 返回成功响应
	token , _:= r.Cookie("mynas_api_token")
	cur, _:= client[token.Value]
	data := &struct {
		Username string `json:"username"`
	} {
		Username: cur.username,
		}
	response := &struct {
		Ret int `json:"ret"`
		Data interface{} `json:"data"`
	}{
		Ret: 0,
		Data: data,
	}
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)


}

func HandleLogin(w http.ResponseWriter, r *http.Request) {
    if r.Method != http.MethodPost {
        http.Error(w, "Invalid request method", http.StatusMethodNotAllowed)
        return
    }

    var req LoginRequest
    if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
        http.Error(w, "Invalid request body", http.StatusBadRequest)
        return
    }

    // 验证用户名和密码
	if !validateCredentials(req.Username, req.Password) {
		response := LoginResponse{Ret: -2}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(response)
		return
	}

    // 生成 API token
    apiToken := generateAPIToken()

    // 设置 cookie
    http.SetCookie(w, &http.Cookie{
        Name:     "mynas_api_token",
        Value:    apiToken,
        Path:     "/",
        Expires:  time.Now().Add(24 * time.Hour),
        HttpOnly: true,
    })
	cur_client := NewUser(apiToken,req.Username);
	client[cur_client.api_token] = cur_client

    // 返回成功响应
	response := LoginResponse{Ret: 0}
    w.Header().Set("Content-Type", "application/json")
    json.NewEncoder(w).Encode(response)
}


func getShadowHash(username string) (string, error) {
	file, err := os.Open("/etc/shadow")
	if err != nil {
		return "", fmt.Errorf("error opening /etc/shadow: %v", err)
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := scanner.Text()
		fields := strings.Split(line, ":")
		if len(fields) > 1 && fields[0] == username {
			return fields[1], nil
		}
	}

	if err := scanner.Err(); err != nil {
		return "", fmt.Errorf("error reading /etc/shadow: %v", err)
	}

	return "", fmt.Errorf("user not found")
}

func validateCredentials(username, password string) bool {
	fmt.Println("username=", username)
	fmt.Println("password=", password)
	shadowHash, err := getShadowHash(username)
	if err != nil {
		fmt.Println("Error getting shadow hash:", err)
		return false
	}
	// 使用系统的账号和密码进行验证
	/*
	// 使用 su 命令进行验证

        cmd := exec.Command("sudo", "-S", "-u", username, "true")
	cmd.Stdin = strings.NewReader(password + "\n")
	err := cmd.Run()
	fmt.Println("check result=", err)
	return err == nil
	*/
	cmd := exec.Command("sh", "/app/scripts/check.sh", password, shadowHash)
	output, err := cmd.Output()
	fmt.Println("check.sh return", string(output))
	if err != nil {
		fmt.Println("Error executing check_login.sh:", err)
		return false
	}
	// 检查脚本输出是否包含 "ok"
	return strings.TrimSpace(string(output)) == "ok"
}

func generateAPIToken() string {
    // 生成一个新的 UUID 作为 API token
    return uuid.New().String()
}

func TokenMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// 检查 cookie 中的 mynas_api_token
		cookie, err := r.Cookie("mynas_api_token")
		if err != nil || !validateToken(cookie.Value) {
			http.Error(w, "Unauthorized", http.StatusUnauthorized)
			return
		}
		next.ServeHTTP(w, r)
	})
}

func validateToken(token string) bool {
	// 这里可以添加更多的 token 验证逻辑，例如检查 token 是否在数据库中
	_, ok:= client[token]
	if(!ok) {
		return false;
	}
	return true
}
