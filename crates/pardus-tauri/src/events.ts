import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { LogEntry } from "./types";

type LogCallback = (entry: LogEntry) => void;

export function onChallengeDetected(
  callback: (info: { url: string; kinds: string[]; risk_score: number }) => void
): Promise<UnlistenFn> {
  return listen("challenge-detected", (event) => {
    callback(event.payload as any);
  });
}

export function onChallengeSolved(
  callback: (info: { url: string }) => void
): Promise<UnlistenFn> {
  return listen("challenge-solved", (event) => {
    callback(event.payload as any);
  });
}

export function onChallengeFailed(
  callback: (url: string, reason: string) => void
): Promise<UnlistenFn> {
  return listen<{ challenge_url: string; reason: string }>(
    "challenge-failed",
    (event) => {
      callback(event.payload.challenge_url, event.payload.reason);
    }
  );
}

export function onBrowserUrlChanged(
  callback: (data: { instance_id: string; url: string }) => void
): Promise<UnlistenFn> {
  return listen("browser-url-changed", (event) => {
    callback(event.payload as any);
  });
}

export function createLogger(container: HTMLElement): LogCallback {
  return (entry: LogEntry) => {
    const div = document.createElement("div");
    div.className = `log-entry ${entry.level}`;
    div.textContent = `[${entry.timestamp}] ${entry.message}`;
    container.appendChild(div);
    container.scrollTop = container.scrollHeight;
  };
}

export function log(level: LogEntry["level"], message: string): LogEntry {
  return {
    level,
    message,
    timestamp: new Date().toISOString().slice(11, 19),
  };
}
