// Copyright 2026 TheHalfMoon contributors
// SPDX-License-Identifier: Apache-2.0

package main

import "encoding/json"

const (
	protocolGeneration = "deskal-computer-host/1"
	hostIdentity = "deskal-windows-computer-host"
	maxRequestBytes = 64 * 1024
	maxResponseBytes = 16 * 1024 * 1024
	maxWindows = 64
	maxTreeNodes = 256
	maxTreeDepth = 8
	maxCaptureBytes = 8 * 1024 * 1024
	maxCaptureWidth = 7680
	maxCaptureHeight = 4320
	workerTimeoutMS = 8000
	workerAttempts = 2
)

var allowedOperations = map[string]struct{}{
	"hello": {},
	"ping": {},
	"list_windows": {},
	"observe_window": {},
	"capture_window": {},
	"shutdown": {},
}

type hostRequest struct {
	ID uint64 `json:"id"`
	Protocol string `json:"protocol"`
	Op string `json:"op"`
	HWND uint64 `json:"hwnd,omitempty"`
	ExpectedPID *uint32 `json:"expected_pid,omitempty"`
	ExpectedStartGeneration *uint64 `json:"expected_start_generation,omitempty"`
	ExpectedWindowNonce *uint64 `json:"expected_window_nonce,omitempty"`
	MaxNodes int `json:"max_nodes,omitempty"`
	MaxDepth int `json:"max_depth,omitempty"`
}

type hostError struct {
	Code string `json:"code"`
	Message string `json:"message"`
}

type processFact struct {
	PID uint32 `json:"pid"`
	ExeName string `json:"exe_name"`
	ExeID string `json:"exe_id"`
	SessionID uint32 `json:"session_id"`
	SessionVerified bool `json:"session_verified"`
	StartGeneration uint64 `json:"start_generation"`
	GenerationSource string `json:"generation_source"`
}

type windowFact struct {
	Process processFact `json:"process"`
	HWND uint64 `json:"hwnd"`
	Title string `json:"title"`
	Class string `json:"class"`
	Visible bool `json:"visible"`
	WindowNonce uint64 `json:"window_nonce"`
}

type elementFact struct {
	RuntimeID string `json:"runtime_id"`
	ControlType string `json:"control_type"`
	AutomationID string `json:"automation_id"`
	Name string `json:"name"`
	Enabled bool `json:"enabled"`
	Selected bool `json:"selected"`
	Toggled bool `json:"toggled"`
	ScrollHorizontalPercent uint8 `json:"scroll_horizontal_percent"`
	ScrollVerticalPercent uint8 `json:"scroll_vertical_percent"`
	Patterns []string `json:"patterns"`
	Value *string `json:"value,omitempty"`
	ValueIsPassword bool `json:"value_is_password"`
	Children []elementFact `json:"children"`
}

type captureFact struct {
	Width uint32 `json:"width"`
	Height uint32 `json:"height"`
	PixelFormat string `json:"pixel_format"`
	PixelsB64 string `json:"pixels_b64"`
}

type hostResponse struct {
	ID uint64 `json:"id"`
	OK bool `json:"ok"`
	Protocol string `json:"protocol"`
	Host string `json:"host"`
	Pong bool `json:"pong,omitempty"`
	Windows []windowFact `json:"windows,omitempty"`
	Elements []elementFact `json:"elements,omitempty"`
	Capture *captureFact `json:"capture,omitempty"`
	Error *hostError `json:"error,omitempty"`
}

func success(id uint64) hostResponse {
	return hostResponse{ID: id, OK: true, Protocol: protocolGeneration, Host: hostIdentity}
}

func failure(id uint64, code, message string) hostResponse {
	return hostResponse{ID: id, OK: false, Protocol: protocolGeneration, Host: hostIdentity, Error: &hostError{Code: code, Message: message}}
}

func validateRequest(req hostRequest) *hostError {
	if req.Protocol != protocolGeneration {
		return &hostError{Code: "invalid_request", Message: "protocol generation mismatch"}
	}
	if _, ok := allowedOperations[req.Op]; !ok {
		return &hostError{Code: "capability_denied", Message: "operation is not exposed by the observation-only host"}
	}
	if req.Op == "observe_window" || req.Op == "capture_window" {
		if req.HWND == 0 {
			return &hostError{Code: "invalid_request", Message: "window handle is required"}
		}
		if req.ExpectedPID == nil || req.ExpectedStartGeneration == nil || req.ExpectedWindowNonce == nil {
			return &hostError{Code: "invalid_request", Message: "exact window binding is required"}
		}
	}
	if req.MaxNodes < 0 || req.MaxDepth < 0 {
		return &hostError{Code: "invalid_request", Message: "tree bounds must be non-negative"}
	}
	return nil
}

func encodedSize(value any) int {
	raw, err := json.Marshal(value)
	if err != nil {
		return maxResponseBytes + 1
	}
	return len(raw)
}
