package main

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestProtocolOperationSurfaceIsExact(t *testing.T) {
	want := []string{"capture_window", "hello", "list_windows", "observe_window", "ping", "shutdown"}
	if len(allowedOperations) != len(want) {
		t.Fatalf("operation count=%d want=%d", len(allowedOperations), len(want))
	}
	for _, op := range want {
		if _, ok := allowedOperations[op]; !ok {
			t.Fatalf("missing operation %q", op)
		}
	}
	for _, forbidden := range []string{"click", "type", "key", "drag", "scroll", "focus", "launch", "activate_window", "set_value", "mouse", "keyboard", "shell", "process"} {
		if _, ok := allowedOperations[forbidden]; ok {
			t.Fatalf("forbidden operation %q exposed", forbidden)
		}
	}
}

func TestProtocolRejectsUnknownAndMalformedRequests(t *testing.T) {
	if err := validateRequest(hostRequest{Protocol: protocolGeneration, Op: "click"}); err == nil {
		t.Fatal("unknown mutation operation accepted")
	}
	if err := validateRequest(hostRequest{Protocol: "other", Op: "ping"}); err == nil {
		t.Fatal("wrong protocol accepted")
	}
	if err := validateRequest(hostRequest{Protocol: protocolGeneration, Op: "capture_window"}); err == nil {
		t.Fatal("capture without HWND accepted")
	}
	if err := validateRequest(hostRequest{Protocol: protocolGeneration, Op: "observe_window", HWND: 1}); err == nil {
		t.Fatal("observation without exact target binding accepted")
	}
	pid := uint32(10)
	start := uint64(20)
	nonce := uint64(30)
	if err := validateRequest(hostRequest{
		Protocol: protocolGeneration,
		Op: "observe_window",
		HWND: 1,
		ExpectedPID: &pid,
		ExpectedStartGeneration: &start,
		ExpectedWindowNonce: &nonce,
	}); err != nil {
		t.Fatalf("complete exact target binding rejected: %+v", err)
	}
}

func TestResponseSchemaContainsNoSecretOrAuthorityFields(t *testing.T) {
	raw, err := json.Marshal(success(1))
	if err != nil {
		t.Fatal(err)
	}
	text := strings.ToLower(string(raw))
	for _, marker := range []string{"token", "secret", "credential", "approval", "lease", "admin"} {
		if strings.Contains(text, marker) {
			t.Fatalf("response unexpectedly contains %q", marker)
		}
	}
}
