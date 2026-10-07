//go:build windows

// Copyright 2026 TheHalfMoon contributors
// SPDX-License-Identifier: Apache-2.0

package main

import "testing"

// SG-000095 uses the independently running Windows Forms probe from SG-000094.
// No test sends input to an arbitrary process, a user window, or a secure surface.

func sg95BoundRequest(t *testing.T, fact windowFact, op string) hostRequest {
	t.Helper()
	req := boundHostRequest(95, op, fact)
	tick, err := currentLastInputTick()
	if err != nil {
		t.Fatalf("read last-input tick: %v", err)
	}
	req.ExpectedLastInputTick = &tick
	return req
}

func sg95ValueNode(elements []elementFact) (elementFact, bool) {
	for _, element := range elements {
		for _, pattern := range element.Patterns {
			if pattern == "Value" && !element.ValueIsPassword {
				return element, true
			}
		}
		if found, ok := sg95ValueNode(element.Children); ok {
			return found, true
		}
	}
	return elementFact{}, false
}

func sg95RequireCompleted(t *testing.T, response hostResponse, action string) {
	t.Helper()
	if !response.OK || response.Action == nil || response.Action.State != "completed" {
		if response.Error != nil {
            t.Fatalf("%s did not complete: %s: %s", action, response.Error.Code, response.Error.Message)
        }
        t.Fatalf("%s did not complete: %+v", action, response)
	}
}

func TestSG95NativeSemanticValueAndInputAgainstDisposableApp(t *testing.T) {
	if err := requireInteractiveStation(); err != nil {
		t.Fatalf("SG95 requires interactive WinSta0: %v", err)
	}
	helper := launchNativeTestWindow(t)
	fact := findNativeTestWindow(t, helper.title)

	observe := boundHostRequest(95, "observe_window", fact)
	observe.MaxNodes, observe.MaxDepth = 64, 4
	observed := nativeObserveWindow(observe)
	if !observed.OK {
		t.Fatalf("observe disposable app: %+v", observed.Error)
	}
	target, ok := sg95ValueNode(observed.Elements)
	if !ok {
		t.Fatal("disposable TextBox did not expose ValuePattern")
	}
	semantic := boundHostRequest(96, "semantic_action", fact)
	semantic.SemanticAction = "set_value"
	semantic.ElementRuntimeID = target.RuntimeID
	semantic.Value = "SG95 semantic value"
	if problem := validateRequest(semantic); problem != nil {
		t.Fatalf("semantic protocol: %s", problem.Message)
	}
	sg95RequireCompleted(t, nativeSemanticAction(semantic), "semantic set_value")
	updated := nativeObserveWindow(observe)
	if !updated.OK {
		t.Fatalf("observe updated target: %+v", updated.Error)
	}
	next, ok := sg95ValueNode(updated.Elements)
	if !ok || next.Value == nil || *next.Value != semantic.Value {
		t.Fatalf("semantic value did not change on disposable app: %+v", next)
	}

	for _, tc := range []struct {
		name string
		makeRequest func(*hostRequest)
	}{
		{"move", func(req *hostRequest) { req.Op = "input_move"; x,y:=45,50; req.X,req.Y=&x,&y }},
		{"click", func(req *hostRequest) { req.Op = "input_click"; x,y:=45,50; req.X,req.Y=&x,&y; req.Button="left"; req.ClickCount=1 }},
		{"double_click", func(req *hostRequest) { req.Op = "input_click"; x,y:=45,50; req.X,req.Y=&x,&y; req.Button="left"; req.ClickCount=2 }},
		{"right_click", func(req *hostRequest) { req.Op = "input_click"; x,y:=45,50; req.X,req.Y=&x,&y; req.Button="right"; req.ClickCount=1 }},
		{"drag", func(req *hostRequest) { req.Op = "input_drag"; x,y,xx,yy:=50,90,100,90; req.X,req.Y,req.ToX,req.ToY=&x,&y,&xx,&yy }},
		{"scroll", func(req *hostRequest) { req.Op="input_scroll"; x,y:=100,110; req.X,req.Y=&x,&y; req.ScrollY=1 }},
		{"type_text", func(req *hostRequest) { req.Op="input_type_text"; req.Text="SG95" }},
		{"key", func(req *hostRequest) { req.Op="input_key"; req.Key="a" }},
		{"hotkey", func(req *hostRequest) { req.Op="input_hotkey"; req.Key="ctrl+a" }},
	} {
		t.Run(tc.name, func(t *testing.T) {
			req := sg95BoundRequest(t, fact, "input_move")
			tc.makeRequest(&req)
			if problem := validateRequest(req); problem != nil {
				t.Fatalf("protocol rejected %s: %s", tc.name, problem.Message)
			}
			sg95RequireCompleted(t, nativeRawInput(req), tc.name)
		})
	}
}

func TestSG95NativeWindowLifecycleOnDisposableApp(t *testing.T) {
	if err := requireInteractiveStation(); err != nil {
		t.Fatalf("SG95 requires interactive WinSta0: %v", err)
	}
	helper := launchNativeTestWindow(t)
	fact := findNativeTestWindow(t, helper.title)
	for _, action := range []string{"focus", "minimize", "restore", "maximize", "restore"} {
		req := sg95BoundRequest(t, fact, "window_action")
		req.WindowAction = action
		if problem := validateRequest(req); problem != nil {
			t.Fatalf("%s protocol: %s", action, problem.Message)
		}
		sg95RequireCompleted(t, nativeWindowAction(req), action)
	}
	closeReq := sg95BoundRequest(t, fact, "window_action")
	closeReq.WindowAction = "close"
	closed := nativeWindowAction(closeReq)
	if !closed.OK || closed.Action == nil || closed.Action.State != "dispatched" {
		t.Fatalf("WM_CLOSE did not report dispatch-only result: %+v", closed)
	}
}

func TestSG95NativeFreshnessAndBoundsFailClosed(t *testing.T) {
	if err := requireInteractiveStation(); err != nil {
		t.Fatalf("SG95 requires interactive WinSta0: %v", err)
	}
	helper := launchNativeTestWindow(t)
	fact := findNativeTestWindow(t, helper.title)

	req := sg95BoundRequest(t, fact, "input_move")
	x,y:=20,20
	req.X,req.Y=&x,&y
	*req.ExpectedPID++
	stale := nativeRawInput(req)
	if stale.OK || stale.Error == nil || stale.Error.Code != "target_stale" {
		t.Fatalf("stale process binding executed: %+v", stale)
	}

	req = sg95BoundRequest(t, fact, "input_move")
	req.X,req.Y=&x,&y
	badTick := *req.ExpectedLastInputTick + 1
	req.ExpectedLastInputTick = &badTick
	cancelled := nativeRawInput(req)
	if cancelled.OK || cancelled.Error == nil || cancelled.Error.Code != "cancelled" {
		t.Fatalf("human-input drift did not cancel before dispatch: %+v", cancelled)
	}

	scroll := sg95BoundRequest(t, fact, "input_scroll")
	scroll.X,scroll.Y=&x,&y
	scroll.ScrollY=maxScrollTicks+1
	if problem := validateRequest(scroll); problem == nil {
		t.Fatal("out-of-bounds wheel input was accepted")
	}
}
