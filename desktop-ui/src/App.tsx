import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

type DaemonStatus = {
  ready: boolean;
  spawned: boolean;
  url: string;
  lastError: string | null;
  logDir: string;
  startupLog: string;
  daemonStdoutLog: string;
  daemonStderrLog: string;
};

async function loadDaemonStatus(): Promise<DaemonStatus> {
  return invoke<DaemonStatus>("get_daemon_status");
}

async function ensureDaemonStarted(): Promise<DaemonStatus> {
  return invoke<DaemonStatus>("ensure_daemon_started");
}

async function openLogDirectory(): Promise<void> {
  return invoke("open_log_directory");
}

export default function App() {
  const [status, setStatus] = useState<"booting" | "ready" | "error">("booting");
  const [details, setDetails] = useState<DaemonStatus | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    let cancelled = false;
    let pollId: number | undefined;

    const refresh = async (starter: () => Promise<DaemonStatus>) => {
      try {
        const next = await starter();
        if (cancelled) {
          return;
        }
        setDetails(next);
        setStatus(next.ready ? "ready" : "booting");
      } catch (error) {
        if (cancelled) {
          return;
        }
        setStatus("error");
        setDetails((current) => ({
          ready: false,
          spawned: current?.spawned ?? false,
          url: current?.url ?? "http://localhost:14564/",
          lastError:
            error instanceof Error ? error.message : "Unknown daemon startup failure",
          logDir: current?.logDir ?? "",
          startupLog: current?.startupLog ?? "",
          daemonStdoutLog: current?.daemonStdoutLog ?? "",
          daemonStderrLog: current?.daemonStderrLog ?? ""
        }));
      }
    };

    void refresh(ensureDaemonStarted);

    pollId = window.setInterval(() => {
      void refresh(loadDaemonStatus);
    }, 2000);

    return () => {
      cancelled = true;
      if (pollId !== undefined) {
        window.clearInterval(pollId);
      }
    };
  }, []);

  const iframeUrl = details?.url ?? "http://localhost:14564/";

  return (
    <main className="app-shell">
      {status === "ready" ? (
        <iframe
          key={reloadKey}
          className="ui-frame"
          src={iframeUrl}
          title="MacXLR Configuration"
        />
      ) : (
        <section className="overlay">
          <div className="overlay-card">
            <p className="overlay-label">{status === "error" ? "Launch Failed" : "Starting"}</p>
            <h1 className="overlay-title">
              {status === "error" ? "MacXLR could not start cleanly." : "Opening MacXLR."}
            </h1>
            <p className="overlay-copy">
              {status === "error"
                ? "The desktop app could not finish starting the background mixer service."
                : "Launching the bundled daemon and waiting for the GoXLR interface."}
            </p>
            {status === "error" ? (
              <>
                <div className="error-panel">
                  <p className="error-message">
                    {details?.lastError ?? "No error details were captured."}
                  </p>
                  <div className="log-block">
                    <p className="log-label">Logs</p>
                    <code>{details?.startupLog}</code>
                    <code>{details?.daemonStdoutLog}</code>
                    <code>{details?.daemonStderrLog}</code>
                  </div>
                </div>
                <div className="button-row">
                  <button
                    type="button"
                    onClick={() => {
                      setStatus("booting");
                      void ensureDaemonStarted()
                        .then((next) => {
                          setDetails(next);
                          setStatus(next.ready ? "ready" : "booting");
                          setReloadKey((value) => value + 1);
                        })
                        .catch((error) => {
                          setStatus("error");
                          setDetails((current) => ({
                            ready: false,
                            spawned: current?.spawned ?? false,
                            url: current?.url ?? "http://localhost:14564/",
                            lastError:
                              error instanceof Error
                                ? error.message
                                : "Unknown daemon startup failure",
                            logDir: current?.logDir ?? "",
                            startupLog: current?.startupLog ?? "",
                            daemonStdoutLog: current?.daemonStdoutLog ?? "",
                            daemonStderrLog: current?.daemonStderrLog ?? ""
                          }));
                        });
                    }}
                  >
                    Retry
                  </button>
                  <button
                    type="button"
                    className="secondary-button"
                    onClick={() => {
                      void openLogDirectory();
                    }}
                  >
                    Open Logs
                  </button>
                </div>
              </>
            ) : (
              <div className="spinner" aria-hidden="true" />
            )}
          </div>
        </section>
      )}
    </main>
  );
}
