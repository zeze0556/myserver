package command

import (
    "bytes"
    "encoding/json"
    "net/http"
    "os/exec"
)

type CommandRequest struct {
    Command string   `json:"command"`
    Args    []string `json:"args,omitempty"`
}

type CommandResponse struct {
    Ret  int    `json:"ret"`
    Data struct {
        Stdout string `json:"stdout"`
        Stderr string `json:"stderr"`
    } `json:"data"`
}

func HandleCommandRun(w http.ResponseWriter, r *http.Request) {
    if r.Method != http.MethodPost {
        http.Error(w, "Invalid request method", http.StatusMethodNotAllowed)
        return
    }

    var req CommandRequest
    if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
        http.Error(w, "Invalid request body", http.StatusBadRequest)
        return
    }

    cmd := exec.Command(req.Command, req.Args...)
    var stdout, stderr bytes.Buffer
    cmd.Stdout = &stdout
    cmd.Stderr = &stderr

    err := cmd.Run()
    retCode := 0
    if err != nil {
        if exitError, ok := err.(*exec.ExitError); ok {
            retCode = exitError.ExitCode()
        } else {
            retCode = -1
        }
    }

    var resp CommandResponse
    resp.Ret = retCode
    resp.Data.Stdout = stdout.String()
    resp.Data.Stderr = stderr.String()

    w.Header().Set("Content-Type", "application/json")
    json.NewEncoder(w).Encode(resp)
}
