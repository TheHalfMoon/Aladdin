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
	attempts := workerAttempts
	if isMutationOperation(req.Op) {
		// A crashed/timed-out mutating worker may have dispatched input before
		// losing its response. Never retry and risk duplicate physical input.
		attempts = 1
	}
	for attempt := 0; attempt < attempts; attempt++ {
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
		resp, decodeErr := decodeWorkerResponse(req, stdout.Bytes())
		if decodeErr != nil {
			last = decodeErr.Error()
			continue
		}
		return resp
	}
	if last == "" {
		last = "native worker unavailable"
	}
	if isMutationOperation(req.Op) {
		return failure(req.ID, "outcome_unknown", last)
	}
	return failure(req.ID, "provider_unavailable", last)
}

func decodeWorkerResponse(req hostRequest, raw []byte) (hostResponse, error) {
	line := bytes.TrimSpace(raw)
	if len(line) == 0 || len(line) > maxResponseBytes {
		return hostResponse{}, errors.New("native worker returned an empty or oversized response")
	}
	if bytes.IndexByte(line, 0) >= 0 {
		return hostResponse{}, errors.New("native worker returned a NUL-contaminated response")
	}
	var resp hostResponse
	if err := json.Unmarshal(line, &resp); err != nil {
		return hostResponse{}, errors.New("native worker returned corrupt JSON")
	}
	if resp.ID != req.ID || resp.Protocol != protocolGeneration || resp.Host != hostIdentity {
		return hostResponse{}, errors.New("native worker response binding mismatch")
	}
	return resp, nil
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
	case "cursor_position":
		resp = nativeCursorPosition(req)
	case "semantic_action":
		resp = nativeSemanticAction(req)
	case "window_action":
		resp = nativeWindowAction(req)
	case "input_move", "input_click", "input_drag", "input_scroll", "input_type_text", "input_key", "input_hotkey":
		resp = nativeRawInput(req)
	default:
		resp = failure(req.ID, "capability_denied", "supervisor-only operation is not executable in a native worker")
	}
	return json.NewEncoder(output).Encode(resp)
}
