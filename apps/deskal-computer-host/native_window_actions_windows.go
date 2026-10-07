//go:build windows

// Copyright 2026 TheHalfMoon contributors
// SPDX-License-Identifier: Apache-2.0
//
// COPY_ADAPT exact-window lifecycle mechanics from opensymph/open-computer-use
// apps/OpenComputerUseWindows/native_win32.go at
// 5b433b98019c18201a15d11e8c3cb0010879a3d8,
// blob e94ce44c7e76ce45035dfe584c883cab16babdf0 (MIT).
//
// Deskal keeps only bounded lifecycle operations on an already-observed exact
// HWND binding. This file adds no process launch, generic messaging, elevation,
// remote-control, or public service authority.

package main

import "golang.org/x/sys/windows"

const (
	swMaximize = 3
	swMinimize = 6
	swRestore  = 9
	wmClose    = 0x0010
)

var (
	procShowWindowLifecycle = user32.NewProc("ShowWindow")
	procIsIconicLifecycle   = user32.NewProc("IsIconic")
	procIsZoomedLifecycle   = user32.NewProc("IsZoomed")
	procPostMessageLifecycle = user32.NewProc("PostMessageW")
)

func showWindowLifecycle(hwnd windows.HWND, command uintptr) {
	_, _, _ = procShowWindowLifecycle.Call(uintptr(hwnd), command)
}

func isIconicLifecycle(hwnd windows.HWND) bool {
	value, _, _ := procIsIconicLifecycle.Call(uintptr(hwnd))
	return value != 0
}

func isZoomedLifecycle(hwnd windows.HWND) bool {
	value, _, _ := procIsZoomedLifecycle.Call(uintptr(hwnd))
	return value != 0
}

func waitWindowState(check func() bool) bool {
	for attempt := 0; attempt < 20; attempt++ {
		if check() {
			return true
		}
		sleepMs(20)
	}
	return check()
}

func nativeWindowAction(req hostRequest) hostResponse {
	if _, _, err := validateBoundWindow(req); err != nil {
		return failure(req.ID, "target_stale", err.Error())
	}
	if err := requireExpectedLastInput(req); err != nil {
		return failure(req.ID, "cancelled", err.Error())
	}
	hwnd := windows.HWND(uintptr(req.HWND))

	switch req.WindowAction {
	case "focus":
		if err := focusBoundWindow(hwnd); err != nil {
			return failure(req.ID, "cancelled", err.Error())
		}
	case "minimize":
		showWindowLifecycle(hwnd, swMinimize)
		if !waitWindowState(func() bool { return isIconicLifecycle(hwnd) }) {
			return failure(req.ID, "postcondition_failed", "target window did not enter minimized state")
		}
	case "maximize":
		showWindowLifecycle(hwnd, swMaximize)
		if !waitWindowState(func() bool { return isZoomedLifecycle(hwnd) }) {
			return failure(req.ID, "postcondition_failed", "target window did not enter maximized state")
		}
	case "restore":
		showWindowLifecycle(hwnd, swRestore)
		if !waitWindowState(func() bool {
			return !isIconicLifecycle(hwnd) && !isZoomedLifecycle(hwnd)
		}) {
			return failure(req.ID, "postcondition_failed", "target window did not enter restored state")
		}
	case "close":
		ok, _, _ := procPostMessageLifecycle.Call(uintptr(hwnd), wmClose, 0, 0)
		if ok == 0 {
			return failure(req.ID, "outcome_unknown", "WM_CLOSE could not be queued to the exact target window")
		}
		resp := success(req.ID)
		resp.Action = &actionFact{State: "dispatched", Action: "window_close"}
		return resp
	default:
		return failure(req.ID, "capability_denied", "window action is not mapped")
	}

	if err := requireExpectedLastInput(req); err != nil && req.WindowAction == "focus" {
		// Focus acquisition may itself race with human input. Report cancellation
		// instead of silently continuing into a broader follow-on action.
		return failure(req.ID, "cancelled", err.Error())
	}
	resp := success(req.ID)
	resp.Action = &actionFact{State: "completed", Action: "window_" + req.WindowAction}
	return resp
}
