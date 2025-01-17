GOARCH=amd64
GOOS=linux
TARGET=game_event

default: mynas_build

mynas_build:
	go build -o mynas_go -ldflags '-s -w -linkmode "external" -extldflags "-static"' -tags musl main.go
