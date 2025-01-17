package main

import (
	"flag"
	_ "context"
	//"encoding/json"
	"github.com/json-iterator/go"
	"fmt"
	"os"
	_ "log"
	"runtime"
	"strings"
	"net/http"
	"io"
	"io/ioutil"
	"github.com/tidwall/gjson"
	"path/filepath"
	"mynas/module/utils"
	"mynas/module/user"
	"mynas/module/config"
	"mynas/module/remoteshell"
	"mynas/module/command"
	"mynas/module/notify"
)
var json = jsoniter.ConfigCompatibleWithStandardLibrary

var myconfig *config.Config

func init(){
	//flag.StringVar(&config_file, "c", "config.json", "config文件")
	var c string
	flag.StringVar(&c, "c", "config.json", "配置文件")
	my_config, _ := config.LoadConfig(c)
	myconfig = my_config
}

func main() {
	flag.Parse()
	defer func() {
		if err := recover(); err != nil{
			fmt.Println("error in main");
			utils.PrintStack()
			fmt.Println(err) // 异常1
		}
	}();
	http.HandleFunc("/api/login", user.HandleLogin);
	http.HandleFunc("/api/notify", notify.HandleNotify);
	http.Handle("/api/", user.TokenMiddleware(http.HandlerFunc(handleAPI)))
	http.Handle("/api/ws/shell", user.TokenMiddleware(http.HandlerFunc(remoteshell.WsHandler)));
	// 文件处理 API
	//http.HandleFunc("/api/file/get", user.TokenMiddleware(http.HandlerFunc(handleFileGet)));
	//http.HandleFunc("/api/file/put", user.TokenMiddleware(http.HandlerFunc(handleFilePut)));
	//http.HandleFunc('/api/userinfo', userinfo);
	//fs := http.FileServer(http.Dir(config_m.Get("checkdir").Str))
	//http.Handle("/fulllog", fs)
	fs := http.FileServer(http.Dir(myconfig.StaticDir))
	http.Handle("/", fs)
	http.ListenAndServe(fmt.Sprintf(":%d",myconfig.Port), nil)
	runtime.GC()
}

func handleAPI(w http.ResponseWriter, r *http.Request) {
	switch {
	case strings.HasPrefix(r.URL.Path, "/api/file/get"):
		handleFileGet(w, r)
	case strings.HasPrefix(r.URL.Path, "/api/file/put"):
		handleFilePut(w, r)
	case strings.HasPrefix(r.URL.Path, "/api/userinfo"):
		user.HandleUserinfo(w, r)
	case strings.HasPrefix(r.URL.Path, "/api/command/run"):
		command.HandleCommandRun(w, r)
		//case strings.HasPrefix(r.URL.Path, "/api/notify"):
		//notify.HandleNotify(w, r)
	default:
		http.Error(w, "Not Found", http.StatusNotFound)
	}
}

func handleFileGet(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Invalid request method", http.StatusMethodNotAllowed)
		return
	}
	body, _ := ioutil.ReadAll(r.Body)
	r.Body.Close()
	if ! gjson.ValidBytes(body){
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		return
	}
	req := gjson.ParseBytes(body)
	filePath := req.Get("filename").String()
	file, err := os.Open(filePath)
	if err != nil {
		http.Error(w, "File not found", http.StatusNotFound)
		return
	}
	defer file.Close()
	w.Header().Set("Content-Disposition", fmt.Sprintf("attachment; filename=%s", filepath.Base(filePath)))
	w.Header().Set("Content-Type", "application/octet-stream")
	if _, err := io.Copy(w, file); err != nil {
		http.Error(w, "Error reading file", http.StatusInternalServerError)
	}
}

func handleFilePut(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPut {
		http.Error(w, "Invalid request method", http.StatusMethodNotAllowed)
		return
	}
	body, _ := ioutil.ReadAll(r.Body)
	r.Body.Close()
	if ! gjson.ValidBytes(body){
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		return
	}
	req := gjson.ParseBytes(body)
	filePath := req.Get("filename").String()
	if filePath == "" {
		http.Error(w, "Missing file_path parameter", http.StatusBadRequest)
		return
	}
	file, err := os.OpenFile(filePath, os.O_RDWR|os.O_CREATE,0644)
	if err != nil {
		http.Error(w, "Error creating file", http.StatusInternalServerError)
		return
	}
	defer file.Close()
	data := req.Get("data").String()

	if _, err := file.WriteString(data); err != nil {
		http.Error(w, "Error writing file", http.StatusInternalServerError)
		return
	}
	response := &struct {
		Ret int `json:"ret"`
	}{
		Ret: 0,
	}
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)
}
