package notify

import (
	"github.com/json-iterator/go"
	"fmt"
	"io/ioutil"
	"net/http"
	"net/smtp"
	"crypto/tls"
	_ "os"
	"path/filepath"
	_ "strings"
	"log"
	_ "reflect"
	"encoding/base64"
	"mynas/module/config"
)

var json = jsoniter.ConfigCompatibleWithStandardLibrary

type NotifyRequest struct {
	Subject string `json:"subject"`
	Message string `json:"message"`
	Attach  string `json:"attach"`
}

func HandleNotify(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Invalid request method", http.StatusMethodNotAllowed)
		return
	}

	var req NotifyRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		response := map[string]interface{}{
			"ret":   -2,
			"error": "Invalid request body",
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(response)
		return
	}

	cfg := config.GetConfig()
	notify, ok := cfg.Notify.(map[string]interface{})
	if !ok {
		response := map[string]interface{}{
			"ret":   -2,
			"error": "Invalid config format",
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(response)
		return
	}
	typeConfig := "mail"
	if v, ok := notify["type"]; ok {
		typeConfig = v.(string)
	}
	if typeConfig != "mail" {
		response := map[string]interface{}{
			"ret":   -2,
			"error": "Unsupported notify type",
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(response)
		return
	}
	if err := sendMail(notify, req.Subject, req.Message, req.Attach); err != nil {
		fmt.Println("error==", err)
		response := map[string]interface{}{
			"ret":   -2,
			"error": "Failed to send mail",
			"err":err,
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(response)
		return
	}

	response := map[string]interface{}{
		"ret": 0,
	}
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)
}


func sendMail(cfg map[string]interface{}, subject, message, attach string) error {
    from := cfg["user"].(string)
    pass := cfg["pass"].(string)
    to := cfg["to"].(string)

    // 读取附件内容
    attachment, err := ioutil.ReadFile(attach)
    if err != nil {
        return fmt.Errorf("failed to read attachment: %v", err)
    }

    // 创建邮件内容
    body := "Subject: " + subject + "\r\n" +
        "To: " + to + "\r\n" +
        "From: " + from + "\r\n" +
        "MIME-Version: 1.0\r\n" +
        "Content-Type: multipart/mixed; boundary=boundary42\r\n" +
        "\r\n" +
        "--boundary42\r\n" +
        "Content-Type: text/plain; charset=UTF-8\r\n" +
        "\r\n" +
        message + "\r\n" +
        "--boundary42\r\n" +
        "Content-Type: application/octet-stream; name=\"" + filepath.Base(attach) + "\"\r\n" +
        "Content-Transfer-Encoding: base64\r\n" +
        "Content-Disposition: attachment; filename=\"" + filepath.Base(attach) + "\"\r\n" +
        "\r\n" +
        encodeBase64(attachment) + "\r\n" +
        "--boundary42--"

	fmt.Println("sendmail body=", body)

    // 设置 SMTP 服务器地址和端口
	smtpHost := cfg["host"].(string)
	smtpPort := fmt.Sprintf("%s", cfg["port"].(string))

    // 连接到 SMTP 服务器
    auth := smtp.PlainAuth("", from, pass, smtpHost)
// TLS config
    tlsconfig := &tls.Config {
        InsecureSkipVerify: true,
        ServerName: smtpHost,
    }

    // Here is the key, you need to call tls.Dial instead of smtp.Dial
    // for smtp servers running on 465 that require an ssl connection
    // from the very beginning (no starttls)
	conn, err := tls.Dial("tcp", smtpHost+":"+smtpPort, tlsconfig)
    if err != nil {
        log.Panic(err)
	    return err
    }

    c, err := smtp.NewClient(conn, smtpHost)
    if err != nil {
        log.Panic(err)
	    return err
    }

    // Auth
    if err = c.Auth(auth); err != nil {
        log.Panic(err)
	    return err
    }

    // To && From
    if err = c.Mail(from); err != nil {
        log.Panic(err)
	    return err
    }

    if err = c.Rcpt(to); err != nil {
        log.Panic(err)
	    return err
    }

    // Data
    w, err := c.Data()
    if err != nil {
        log.Panic(err)
	    return err
    }

    _, err = w.Write([]byte(body))
    if err != nil {
        log.Panic(err)
	    return err
    }

    err = w.Close()
    if err != nil {
        log.Panic(err)
	    return err
    }

    c.Quit()
	/*
    err = smtp.SendMail(smtpHost+":"+smtpPort, auth, from, strings.Split(to, ","), []byte(body))
    if err != nil {
        return fmt.Errorf("failed to send mail: %v", err)
    }
	*/

    return nil
}

func encodeBase64(data []byte) string {
	return base64.StdEncoding.EncodeToString(data)
}

