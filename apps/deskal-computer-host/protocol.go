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
	maxInputTextBytes = 16 * 1024
	maxKeyBytes = 64
	maxScrollTicks = 20
	workerTimeoutMS = 8000
	workerAttempts = 2
)

var allowedOperations = map[string]struct{}{
	"hello": {},
	"ping": {},
	"list_windows": {},
	"observe_window": {},
	"capture_window": {},
	"cursor_position": {},
	"semantic_action": {},
	"window_action": {},
	"input_move": {},
	"input_click": {},
	"input_drag": {},
	"input_scroll": {},
	"input_type_text": {},
	"input_key": {},
	"input_hotkey": {},
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
	ElementRuntimeID string `json:"element_runtime_id,omitempty"`
	SemanticAction string `json:"semantic_action,omitempty"`
	WindowAction string `json:"window_action,omitempty"`
	Value string `json:"value,omitempty"`
	X *int `json:"x,omitempty"`
	Y *int `json:"y,omitempty"`
	ToX *int `json:"to_x,omitempty"`
	ToY *int `json:"to_y,omitempty"`
	Button string `json:"button,omitempty"`
	ClickCount int `json:"click_count,omitempty"`
	ScrollX int `json:"scroll_x,omitempty"`
	ScrollY int `json:"scroll_y,omitempty"`
	Text string `json:"text,omitempty"`
	Key string `json:"key,omitempty"`
	ExpectedLastInputTick *uint32 `json:"expected_last_input_tick,omitempty"`
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

type cursorFact struct {
	X int `json:"x"`
	Y int `json:"y"`
	ScreenWidth int `json:"screen_width"`
	ScreenHeight int `json:"screen_height"`
	LastInputTick uint32 `json:"last_input_tick"`
}

type actionFact struct {
	State string `json:"state"`
	Action string `json:"action"`
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
	Cursor *cursorFact `json:"cursor,omitempty"`
	Action *actionFact `json:"action,omitempty"`
	Error *hostError `json:"error,omitempty"`
}

func success(id uint64) hostResponse {
	return hostResponse{ID: id, OK: true, Protocol: protocolGeneration, Host: hostIdentity}
}

func failure(id uint64, code, message string) hostResponse {
	return hostResponse{ID: id, OK: false, Protocol: protocolGeneration, Host: hostIdentity, Error: &hostError{Code: code, Message: message}}
}

func isMutationOperation(op string) bool {
	switch op {
	case "semantic_action", "window_action", "input_move", "input_click", "input_drag", "input_scroll",
		"input_type_text", "input_key", "input_hotkey":
		return true
	default:
		return false
	}
}

func requiresWindowBinding(op string) bool {
	return op == "observe_window" || op == "capture_window" || op == "semantic_action" ||
		isMutationOperation(op)
}

func validateRequest(req hostRequest) *hostError {
	if req.Protocol != protocolGeneration {
		return &hostError{Code: "invalid_request", Message: "protocol generation mismatch"}
	}
	if _, ok := allowedOperations[req.Op]; !ok {
		return &hostError{Code: "capability_denied", Message: "operation is not exposed by the private host"}
	}
	if requiresWindowBinding(req.Op) {
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
	if len(req.Text) > maxInputTextBytes || len(req.Value) > maxInputTextBytes {
		return &hostError{Code: "invalid_request", Message: "text input exceeds the bounded limit"}
	}
	if len(req.Key) > maxKeyBytes {
		return &hostError{Code: "invalid_request", Message: "key input exceeds the bounded limit"}
	}
	if isMutationOperation(req.Op) && req.Op != "semantic_action" && req.ExpectedLastInputTick == nil {
		return &hostError{Code: "invalid_request", Message: "raw input requires a fresh human-input tick binding"}
	}
	switch req.Op {
	case "window_action":
		switch req.WindowAction {
		case "focus", "minimize", "maximize", "restore", "close":
		default:
			return &hostError{Code: "invalid_request", Message: "unknown window action"}
		}
	case "semantic_action":
		switch req.SemanticAction {
		case "invoke", "set_value", "select", "toggle", "scroll":
		default:
			return &hostError{Code: "invalid_request", Message: "unknown semantic action"}
		}
		if req.ElementRuntimeID == "" || len(req.ElementRuntimeID) > 512 {
			return &hostError{Code: "invalid_request", Message: "exact element runtime id is required"}
		}
		if req.SemanticAction == "scroll" && (req.ScrollX < -maxScrollTicks || req.ScrollX > maxScrollTicks || req.ScrollY < -maxScrollTicks || req.ScrollY > maxScrollTicks) {
			return &hostError{Code: "invalid_request", Message: "semantic scroll exceeds the bounded limit"}
		}
	case "input_move":
		if req.X == nil || req.Y == nil {
			return &hostError{Code: "invalid_request", Message: "window-relative x and y are required"}
		}
	case "input_drag":
		if req.X == nil || req.Y == nil || req.ToX == nil || req.ToY == nil {
			return &hostError{Code: "invalid_request", Message: "drag requires window-relative start and end coordinates"}
		}
	case "input_click":
		if req.X == nil || req.Y == nil {
			return &hostError{Code: "invalid_request", Message: "window-relative x and y are required"}
		}
		if req.Button != "left" && req.Button != "right" && req.Button != "middle" {
			return &hostError{Code: "invalid_request", Message: "mouse button must be left, right, or middle"}
		}
		if req.ClickCount != 1 && req.ClickCount != 2 {
			return &hostError{Code: "invalid_request", Message: "click_count must be 1 or 2"}
		}
	case "input_scroll":
		if req.X == nil || req.Y == nil {
			return &hostError{Code: "invalid_request", Message: "scroll requires window-relative x and y"}
		}
		if req.ScrollX < -maxScrollTicks || req.ScrollX > maxScrollTicks || req.ScrollY < -maxScrollTicks || req.ScrollY > maxScrollTicks || (req.ScrollX == 0 && req.ScrollY == 0) {
			return &hostError{Code: "invalid_request", Message: "scroll ticks are missing or out of bounds"}
		}
	case "input_type_text":
		if req.Text == "" {
			return &hostError{Code: "invalid_request", Message: "text is required"}
		}
	case "input_key", "input_hotkey":
		if req.Key == "" {
			return &hostError{Code: "invalid_request", Message: "key or hotkey is required"}
		}
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
