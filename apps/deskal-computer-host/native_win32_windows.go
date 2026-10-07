//go:build windows

// Copyright 2026 TheHalfMoon contributors
// SPDX-License-Identifier: Apache-2.0
//
// Observation/capture subset adapted from opensymph/open-computer-use
// apps/OpenComputerUseWindows/native_win32.go at
// 5b433b98019c18201a15d11e8c3cb0010879a3d8 (MIT).
// All input, message mutation, focus, activation, and launch helpers are
// intentionally absent from SG-000094.

package main

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"strings"
	"unsafe"

	"golang.org/x/sys/windows"
)

const (
	processQueryLimitedInformation = 0x1000
	uoiName = 2
	dwmwaCloaked = 14
	pwRenderFullContent = 2
	srcCopy = 0x00CC0020
	biRGB = 0
	dibRGBColors = 0
)

var (
	user32 = windows.NewLazySystemDLL("user32.dll")
	gdi32 = windows.NewLazySystemDLL("gdi32.dll")
	dwmapi = windows.NewLazySystemDLL("dwmapi.dll")
	kernel32 = windows.NewLazySystemDLL("kernel32.dll")
	ole32 = windows.NewLazySystemDLL("ole32.dll")

	procEnumWindows = user32.NewProc("EnumWindows")
	procIsWindowVisible = user32.NewProc("IsWindowVisible")
	procGetWindowTextLength = user32.NewProc("GetWindowTextLengthW")
	procGetWindowText = user32.NewProc("GetWindowTextW")
	procGetClassName = user32.NewProc("GetClassNameW")
	procGetWindowThreadProcessID = user32.NewProc("GetWindowThreadProcessId")
	procGetWindowRect = user32.NewProc("GetWindowRect")
	procGetWindowDC = user32.NewProc("GetWindowDC")
	procReleaseDC = user32.NewProc("ReleaseDC")
	procPrintWindow = user32.NewProc("PrintWindow")
	procGetProcessWindowStation = user32.NewProc("GetProcessWindowStation")
	procGetUserObjectInformation = user32.NewProc("GetUserObjectInformationW")

	procCreateCompatibleDC = gdi32.NewProc("CreateCompatibleDC")
	procCreateCompatibleBitmap = gdi32.NewProc("CreateCompatibleBitmap")
	procSelectObject = gdi32.NewProc("SelectObject")
	procDeleteDC = gdi32.NewProc("DeleteDC")
	procDeleteObject = gdi32.NewProc("DeleteObject")
	procBitBlt = gdi32.NewProc("BitBlt")
	procGetDIBits = gdi32.NewProc("GetDIBits")

	procDwmGetWindowAttribute = dwmapi.NewProc("DwmGetWindowAttribute")
	procQueryFullProcessImageName = kernel32.NewProc("QueryFullProcessImageNameW")
	procGetProcessTimes = kernel32.NewProc("GetProcessTimes")
	procProcessIdToSessionId = kernel32.NewProc("ProcessIdToSessionId")
	procCoInitializeEx = ole32.NewProc("CoInitializeEx")
	procCoCreateInstance = ole32.NewProc("CoCreateInstance")
)

var protectedExecutables = []string{
	"qdral.exe",
	"qdrald.exe",
	"deskal.exe",
	"deskal-computer-host.exe",
	"credentialuibroker.exe",
	"consent.exe",
	"logonui.exe",
}

type windowRect struct { Left, Top, Right, Bottom int32 }

type bitmapInfoHeader struct {
	Size uint32
	Width int32
	Height int32
	Planes uint16
	BitCount uint16
	Compression uint32
	SizeImage uint32
	XPelsPerMeter int32
	YPelsPerMeter int32
	ClrUsed uint32
	ClrImportant uint32
}

type rgbQuad struct { Blue, Green, Red, Reserved byte }
type bitmapInfo struct { Header bitmapInfoHeader; Colors [1]rgbQuad }

func sleepMs(ms int) { windows.SleepEx(uint32(ms), false) }

func clsidFromString(value string) (windows.GUID, error) {
	return windows.GUIDFromString(value)
}

func oleCreateInstance(clsidText, iidText string) (unsafe.Pointer, error) {
	clsid, err := clsidFromString(clsidText)
	if err != nil { return nil, err }
	iid, err := clsidFromString(iidText)
	if err != nil { return nil, err }
	var out unsafe.Pointer
	hr, _, _ := procCoCreateInstance.Call(
		uintptr(unsafe.Pointer(&clsid)), 0, 1,
		uintptr(unsafe.Pointer(&iid)), uintptr(unsafe.Pointer(&out)))
	if int32(hr) < 0 || out == nil {
		return nil, fmt.Errorf("CoCreateInstance failed: 0x%08x", uint32(hr))
	}
	return out, nil
}

func oleQueryInterface(self unsafe.Pointer, iidText string) (unsafe.Pointer, error) {
	iid, err := clsidFromString(iidText)
	if err != nil { return nil, err }
	var out unsafe.Pointer
	hr, _, _ := vtableCall(self, 0, uintptr(unsafe.Pointer(&iid)), uintptr(unsafe.Pointer(&out)))
	if int32(hr) < 0 || out == nil {
		return nil, fmt.Errorf("QueryInterface failed: 0x%08x", uint32(hr))
	}
	return out, nil
}

func oleRelease(self unsafe.Pointer) {
	if self != nil { _, _, _ = vtableCall(self, 2) }
}

func vtableCall(self unsafe.Pointer, slot int, args ...uintptr) (uintptr, uintptr, error) {
	if self == nil { return 0, 0, errors.New("nil COM pointer") }
	vtable := *(*uintptr)(self)
	fn := *(*uintptr)(unsafe.Pointer(vtable + uintptr(slot)*unsafe.Sizeof(uintptr(0))))
	all := make([]uintptr, 0, len(args)+1)
	all = append(all, uintptr(self))
	all = append(all, args...)
	r1, r2, callErr := windows.SyscallN(append([]uintptr{fn}, all...)...)
	return r1, r2, callErr
}

func utf16String(ptr *uint16, max int) string {
	if ptr == nil || max <= 0 { return "" }
	buf := unsafe.Slice(ptr, max)
	end := 0
	for end < len(buf) && buf[end] != 0 { end++ }
	return windows.UTF16ToString(buf[:end])
}

func currentSessionID() (uint32, bool) {
	return sessionIDForPID(uint32(windows.GetCurrentProcessId()))
}

func sessionIDForPID(pid uint32) (uint32, bool) {
	var session uint32
	ok, _, _ := procProcessIdToSessionId.Call(uintptr(pid), uintptr(unsafe.Pointer(&session)))
	return session, ok != 0
}

func requireInteractiveStation() error {
	station, _, _ := procGetProcessWindowStation.Call()
	if station == 0 { return errors.New("interactive window station is unavailable") }
	var needed uint32
	name := make([]uint16, 64)
	ok, _, _ := procGetUserObjectInformation.Call(
		station, uoiName, uintptr(unsafe.Pointer(&name[0])),
		uintptr(len(name)*2), uintptr(unsafe.Pointer(&needed)))
	if ok == 0 { return errors.New("window station name is unreadable") }
	if !strings.EqualFold(windows.UTF16ToString(name), "WinSta0") {
		return errors.New("host is not attached to WinSta0")
	}
	return nil
}

func isProtectedExecutable(name string) bool {
	for _, item := range protectedExecutables {
		if strings.EqualFold(name, item) { return true }
	}
	return false
}

func processFactByPID(pid uint32) (processFact, bool) {
	sessionID, ok := sessionIDForPID(pid)
	if !ok { return processFact{}, false }
	handle, err := windows.OpenProcess(processQueryLimitedInformation, false, pid)
	if err != nil { return processFact{}, false }
	defer windows.CloseHandle(handle)

	buf := make([]uint16, 1024)
	size := uint32(len(buf))
	okCall, _, _ := procQueryFullProcessImageName.Call(
		uintptr(handle), 0, uintptr(unsafe.Pointer(&buf[0])), uintptr(unsafe.Pointer(&size)))
	if okCall == 0 || size == 0 { return processFact{}, false }
	path := windows.UTF16ToString(buf[:size])
	parts := strings.FieldsFunc(path, func(r rune) bool { return r == '\\' || r == '/' })
	if len(parts) == 0 { return processFact{}, false }
	exe := parts[len(parts)-1]
	if isProtectedExecutable(exe) { return processFact{}, false }

	var creation, exit, kernel, user windows.Filetime
	okCall, _, _ = procGetProcessTimes.Call(
		uintptr(handle),
		uintptr(unsafe.Pointer(&creation)), uintptr(unsafe.Pointer(&exit)),
		uintptr(unsafe.Pointer(&kernel)), uintptr(unsafe.Pointer(&user)))
	if okCall == 0 { return processFact{}, false }
	start := uint64(creation.HighDateTime)<<32 | uint64(creation.LowDateTime)
	sum := sha256.Sum256([]byte("deskal-computer-host/exe/v1|" + strings.ToLower(path)))
	return processFact{
		PID: pid,
		ExeName: exe,
		ExeID: hex.EncodeToString(sum[:16]),
		SessionID: sessionID,
		SessionVerified: true,
		StartGeneration: start,
		GenerationSource: "win32-creation-time",
	}, true
}

func windowText(hwnd windows.HWND) string {
	length, _, _ := procGetWindowTextLength.Call(uintptr(hwnd))
	if length == 0 { return "" }
	buf := make([]uint16, int(length)+1)
	n, _, _ := procGetWindowText.Call(uintptr(hwnd), uintptr(unsafe.Pointer(&buf[0])), uintptr(len(buf)))
	if n == 0 { return "" }
	return windows.UTF16ToString(buf[:n])
}

func className(hwnd windows.HWND) string {
	buf := make([]uint16, 256)
	n, _, _ := procGetClassName.Call(uintptr(hwnd), uintptr(unsafe.Pointer(&buf[0])), uintptr(len(buf)))
	if n == 0 { return "" }
	return windows.UTF16ToString(buf[:n])
}

func ownerPID(hwnd windows.HWND) uint32 {
	var pid uint32
	_, _, _ = procGetWindowThreadProcessID.Call(uintptr(hwnd), uintptr(unsafe.Pointer(&pid)))
	return pid
}

func isCloaked(hwnd windows.HWND) bool {
	var cloaked uint32
	hr, _, _ := procDwmGetWindowAttribute.Call(
		uintptr(hwnd), dwmwaCloaked, uintptr(unsafe.Pointer(&cloaked)), unsafe.Sizeof(cloaked))
	return int32(hr) >= 0 && cloaked != 0
}

func getWindowRect(hwnd windows.HWND) (windowRect, bool) {
	var rect windowRect
	ok, _, _ := procGetWindowRect.Call(uintptr(hwnd), uintptr(unsafe.Pointer(&rect)))
	return rect, ok != 0 && rect.Right > rect.Left && rect.Bottom > rect.Top
}

func enumerateWindows() ([]windowFact, error) {
	if err := requireInteractiveStation(); err != nil { return nil, err }
	ownSession, ok := currentSessionID()
	if !ok { return nil, errors.New("current session id is unreadable") }
	facts := make([]windowFact, 0, maxWindows)
	cb := windows.NewCallback(func(raw, _ uintptr) uintptr {
		if len(facts) >= maxWindows { return 0 }
		hwnd := windows.HWND(raw)
		visible, _, _ := procIsWindowVisible.Call(raw)
		if visible == 0 || isCloaked(hwnd) { return 1 }
		title := strings.TrimSpace(windowText(hwnd))
		if title == "" { return 1 }
		pid := ownerPID(hwnd)
		if pid == 0 || pid == uint32(windows.GetCurrentProcessId()) { return 1 }
		process, ok := processFactByPID(pid)
		if !ok || process.SessionID != ownSession { return 1 }
		class := className(hwnd)
		nonceInput := fmt.Sprintf("%d|%d|%s", pid, process.StartGeneration, class)
		nonceHash := sha256.Sum256([]byte(nonceInput))
		nonce := *(*uint64)(unsafe.Pointer(&nonceHash[0]))
		facts = append(facts, windowFact{
			Process: process, HWND: uint64(raw), Title: title, Class: class,
			Visible: true, WindowNonce: nonce,
		})
		return 1
	})
	okCall, _, _ := procEnumWindows.Call(cb, 0)
	if okCall == 0 { return nil, errors.New("EnumWindows failed") }
	return facts, nil
}

func validateWindow(hwnd windows.HWND) (processFact, windowRect, error) {
	if err := requireInteractiveStation(); err != nil { return processFact{}, windowRect{}, err }
	if !windows.IsWindow(hwnd) { return processFact{}, windowRect{}, errors.New("window is stale") }
	visible, _, _ := procIsWindowVisible.Call(uintptr(hwnd))
	if visible == 0 || isCloaked(hwnd) { return processFact{}, windowRect{}, errors.New("window is not visible") }
	pid := ownerPID(hwnd)
	process, ok := processFactByPID(pid)
	if !ok { return processFact{}, windowRect{}, errors.New("window owner is excluded or unreadable") }
	ownSession, ok := currentSessionID()
	if !ok || process.SessionID != ownSession { return processFact{}, windowRect{}, errors.New("window is outside the interactive session") }
	rect, ok := getWindowRect(hwnd)
	if !ok { return processFact{}, windowRect{}, errors.New("window geometry is unreadable") }
	return process, rect, nil
}

func capturePrintWindowPixels(hwnd windows.HWND) ([]byte, int, int, error) {
	rect, ok := getWindowRect(hwnd)
	if !ok { return nil, 0, 0, errors.New("PrintWindow: invalid window bounds") }
	return captureWithWindowDC(hwnd, rect, true)
}

func captureWindowGDIPixels(hwnd windows.HWND) ([]byte, int, int, error) {
	rect, ok := getWindowRect(hwnd)
	if !ok { return nil, 0, 0, errors.New("GDI: invalid window bounds") }
	return captureWithWindowDC(hwnd, rect, false)
}

func captureWithWindowDC(hwnd windows.HWND, rect windowRect, usePrint bool) ([]byte, int, int, error) {
	width, height := int(rect.Right-rect.Left), int(rect.Bottom-rect.Top)
	if width <= 0 || height <= 0 { return nil, 0, 0, errors.New("capture geometry is empty") }
	windowDC, _, _ := procGetWindowDC.Call(uintptr(hwnd))
	if windowDC == 0 { return nil, 0, 0, errors.New("GetWindowDC failed") }
	defer procReleaseDC.Call(uintptr(hwnd), windowDC)

	memDC, _, _ := procCreateCompatibleDC.Call(windowDC)
	if memDC == 0 { return nil, 0, 0, errors.New("CreateCompatibleDC failed") }
	defer procDeleteDC.Call(memDC)

	bitmap, _, _ := procCreateCompatibleBitmap.Call(windowDC, uintptr(width), uintptr(height))
	if bitmap == 0 { return nil, 0, 0, errors.New("CreateCompatibleBitmap failed") }
	defer procDeleteObject.Call(bitmap)

	old, _, _ := procSelectObject.Call(memDC, bitmap)
	defer procSelectObject.Call(memDC, old)

	if usePrint {
		okCall, _, _ := procPrintWindow.Call(uintptr(hwnd), memDC, pwRenderFullContent)
		if okCall == 0 { return nil, 0, 0, errors.New("PrintWindow failed") }
	} else {
		okCall, _, _ := procBitBlt.Call(memDC, 0, 0, uintptr(width), uintptr(height), windowDC, 0, 0, srcCopy)
		if okCall == 0 { return nil, 0, 0, errors.New("BitBlt failed") }
	}
	return dibPixels(memDC, bitmap, width, height)
}

func dibPixels(dc, bitmap uintptr, width, height int) ([]byte, int, int, error) {
	info := bitmapInfo{Header: bitmapInfoHeader{
		Size: uint32(unsafe.Sizeof(bitmapInfoHeader{})), Width: int32(width),
		Height: -int32(height), Planes: 1, BitCount: 32, Compression: biRGB,
	}}
	pixels := make([]byte, width*height*4)
	lines, _, _ := procGetDIBits.Call(
		dc, bitmap, 0, uintptr(height), uintptr(unsafe.Pointer(&pixels[0])),
		uintptr(unsafe.Pointer(&info)), dibRGBColors)
	if int(lines) != height { return nil, 0, 0, errors.New("GetDIBits failed") }
	return pixels, width, height, nil
}

func isBlankPixels(pixels []byte) bool {
	if len(pixels) == 0 { return true }
	step := len(pixels)/128
	if step < 4 { step = 4 }
	for i := 0; i+2 < len(pixels); i += step {
		if pixels[i] != 0 || pixels[i+1] != 0 || pixels[i+2] != 0 { return false }
	}
	return true
}

func bgraToRGBA(pixels []byte) []byte {
	out := make([]byte, len(pixels))
	copy(out, pixels)
	for i := 0; i+3 < len(out); i += 4 {
		out[i], out[i+2] = out[i+2], out[i]
	}
	return out
}
