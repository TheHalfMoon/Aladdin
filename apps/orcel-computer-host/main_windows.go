//go:build windows

// Copyright 2026 TheHalfMoon contributors
// SPDX-License-Identifier: Apache-2.0
//
// SG-000094 supervisor. The long-lived process never performs a native
// observation itself. Every list/tree/capture request is delegated to a
// one-shot sacrificial worker and there is no in-process fallback.

package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
	"os"
)

func main() {
	if len(os.Args) == 2 && os.Args[1] == "--worker" {
		if err := workerMain(os.Stdin, os.Stdout); err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(2)
		}
		return
	}
	if err := supervisorMain(os.Stdin, os.Stdout); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func supervisorMain(input io.Reader, output io.Writer) error {
	scanner := bufio.NewScanner(input)
	scanner.Buffer(make([]byte, 4096), maxRequestBytes)
	encoder := json.NewEncoder(output)
	for scanner.Scan() {
		var req hostRequest
		if err := json.Unmarshal(scanner.Bytes(), &req); err != nil {
			if err := encoder.Encode(failure(0, "invalid_request", "request is not valid JSON")); err != nil {
				return err
			}
			continue
		}
		if problem := validateRequest(req); problem != nil {
			if err := encoder.Encode(failure(req.ID, problem.Code, problem.Message)); err != nil {
				return err
			}
			continue
		}
		var resp hostResponse
		switch req.Op {
		case "hello":
			resp = success(req.ID)
		case "ping":
			resp = success(req.ID)
			resp.Pong = true
		case "shutdown":
			resp = success(req.ID)
			if err := encoder.Encode(resp); err != nil {
				return err
			}
			return nil
		default:
			resp = runSacrificialWorker(req)
		}
		if encodedSize(resp) > maxResponseBytes {
			resp = failure(req.ID, "output_limit", "host response exceeds the bounded private IPC size")
		}
		if err := encoder.Encode(resp); err != nil {
			return err
		}
	}
	return scanner.Err()
}
