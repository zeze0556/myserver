package config

import ("os"
	"fmt"
	"github.com/json-iterator/go"
)
var json = jsoniter.ConfigCompatibleWithStandardLibrary

type Config struct {
	//config_str string
	Port int
	StaticDir string
	Notify interface{}
	//Get gjson.Result
}


var myconfig *Config

func LoadConfig(filename string) (*Config, error) {
	file, err := os.Open(filename)
	if err != nil {
		return nil, fmt.Errorf("error opening config file: %v", err)
	}
	defer file.Close()

	decoder := json.NewDecoder(file)
	config := &Config{
		Port:     8080,       // 默认值
		StaticDir: "/app/dist",  // 默认值
	}
	if err := decoder.Decode(config); err != nil {
		return nil, fmt.Errorf("error decoding config file: %v", err)
	}
	myconfig = config;

	return config, nil
}

func GetConfig() *Config{
	return myconfig;
}
