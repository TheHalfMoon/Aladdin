//go:build windows

// Copyright 2026 TheHalfMoon contributors
// SPDX-License-Identifier: Apache-2.0
//
// Adapted from opensymph/open-computer-use
// apps/OpenComputerUseWindows/native_op_client.go at
// 5b433b98019c18201a15d11e8c3cb0010879a3d8 (MIT).
// Deskal changes: one request per worker, strict observation-only allowlist,
// scrubbed environment, bounded retries, typed fail-closed errors, and no
// in-process fallback.

package main

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"io"
	"os"
	"os/exec"
	"strings"
	"time"
)

func scrubbedWorkerEnv() []string {
	allow := []string{"SystemRoot", "WINDIR", "COMSPEC", "TEMP", "TMP", "PATH"}
	out := make([]string, 0, len(allow))
	for _, key := range allow {
		if value, ok := os.LookupEnv(key); ok && strings.IndexByte(value, 0) < 0 {
			out = append(out, key+"="+value)
		}
	}
	return out
}

func runSacrificialWorker(req hostRequest) hostResponse {
	payload, err := json.Marshal(req)
	if err != nil {
		return failure(req.ID, "invalid_request", "request serialization failed")
	}
	var last string
	for attempt := 0; attempt < workerAttempts; attempt++ {
		ctx, cancel := context.WithTimeout(context.Background(), workerTimeoutMS*time.Millisecond)
		cmd := exec.CommandContext(ctx, os.Args[0], "--worker")
		cmd.Env = scrubbedWorkerEnv()
		cmd.Stdin = bytes.NewReader(append(payload, byte('\n')))
		var stdout bytes.Buffer
		cmd.Stdout = &stdout
		runErr := cmd.Run()
		cancel()
		if errors.Is(ctx.Err(), context.DeadlineExceeded) {
			last = "native worker timed out"
			continue
		}
		if runErr != nil {
			last = "native worker exited unsuccessfully"
			continue
		}
		line := bytes.TrimSpace(stdout.Bytes())
		if len(line) == 0 || len(line) > maxResponseBytes {
			last = "native worker returned an empty or oversized response"
			continue
		}
		var resp hostResponse
		if err := json.Unmarshal(line, &resp); err != nil {
			last = "native worker returned corrupt JSON"
			continue
		}
		if resp.ID != req.ID || resp.Protocol != protocolGeneration || resp.Host != hostIdentity {
			last = "native worker response binding mismatch"
			continue
		}
		return resp
	}
	if last == "" {
		last = "native worker unavailable"
	}
	return failure(req.ID, "provider_unavailable", last)
}

func workerMain(input io.Reader, output io.Writer) error {
	decoder := json.NewDecoder(io.LimitReader(input, maxRequestBytes))
	var req hostRequest
	if err := decoder.Decode(&req); err != nil {
		return err
	}
	if problem := validateRequest(req); problem != nil {
		return json.NewEncoder(output).Encode(failure(req.ID, problem.Code, problem.Message))
	}
	var resp hostResponse
	switch req.Op {
	case "list_windows":
		resp = nativeListWindows(req)
	case "observe_window":
		resp = nativeObserveWindow(req)
	case "capture_window":
		resp = nativeCaptureWindow(req)
	default:
		resp = failure(req.ID, "capability_denied", "supervisor-only operation is not executable in a native worker")
	}
	return json.NewEncoder(output).Encode(resp)
}
