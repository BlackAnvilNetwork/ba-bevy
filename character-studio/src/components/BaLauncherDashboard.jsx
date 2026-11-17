import React, { useEffect, useState } from "react"
import "./BaLauncherDashboard.css"

/**
 * Black Anvil's native entry surface. Character Studio stays mounted in the
 * *same Tauri application*; the game is a separately launched Bevy process.
 * This screen deliberately does not pretend that Discord OAuth is available.
 */
export default function BaLauncherDashboard({ onCustomize }) {
  const [character, setCharacter] = useState(null)
  const [error, setError] = useState("")
  const [status, setStatus] = useState("Loading local character…")
  const [busy, setBusy] = useState(false)
  const [multiplayer, setMultiplayer] = useState(false)

  const invoke = window.__TAURI__?.tauri?.invoke || window.__TAURI__?.invoke

  useEffect(() => {
    let active = true
    if (typeof invoke !== "function") {
      setStatus("Open Black Anvil through its native launcher to play.")
      return
    }
    invoke("get_draft").then((raw) => {
      if (!active) return
      if (!raw) {
        setCharacter(null)
        setStatus("No character saved yet")
        return
      }
      const draft = JSON.parse(raw)
      if (draft.schemaVersion !== 1 || !draft.characterId) {
        throw new Error("Character format is not supported")
      }
      setCharacter(draft)
      setStatus("Saved locally · development preview")
    }).catch((e) => {
      if (active) {
        setError(String(e?.message || e))
        setStatus("Character unavailable")
      }
    })
    return () => { active = false }
  }, [invoke])

  const play = async () => {
    if (busy || !character || typeof invoke !== "function") return
    setBusy(true)
    setError("")
    try {
      const result = await invoke("launch_game", { multiplayer })
      setStatus(result.alreadyRunning
        ? "Black Anvil is already running"
        : `Launched ${result.mode === "loopback-zone" ? "loopback multiplayer" : "local game"} (PID ${result.pid})`)
    } catch (e) {
      setError(String(e?.message || e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <main className="ba-shell">
      <header className="ba-top">
        <div className="ba-brand"><span className="ba-emblem">◆</span><span>BLACK ANVIL</span></div>
        <span className="ba-build">DEVELOPMENT BUILD</span>
      </header>
      <div className="ba-content">
        <div className="ba-eyebrow">YOUR WORLD AWAITS</div>
        <h1>Forged for adventure.</h1>
        <p className="ba-intro">Shape your adventurer, then enter the ruins of Greymere. Your character is created here in the launcher and played in native Bevy.</p>
        <section className="ba-panel" aria-label="Your character">
          <div className="ba-panel-header"><span>CHARACTER</span><span className="ba-state">{status}</span></div>
          <div className="ba-player-row">
            <div className="ba-avatar">◈</div>
            <div className="ba-player-details">
              <h2>{character?.name || "Create your adventurer"}</h2>
              <p>{character ? `${character.startingJob} · ${character.manifestId} · revision ${character.revision}` : "Customize your appearance, name and starting job."}</p>
            </div>
          </div>
          <div className="ba-actions">
            <button type="button" className="ba-secondary" onClick={onCustomize}>{character ? "Customize character" : "Create character"}</button>
            <button type="button" className="ba-primary" onClick={play} disabled={!character || busy}>{busy ? "Launching…" : "Play Black Anvil"}</button>
          </div>
          <label className="ba-multi"><input type="checkbox" checked={multiplayer} onChange={(e) => setMultiplayer(e.target.checked)} /> Connect to local zone server (developer multiplayer preview)</label>
          {error && <p role="alert" className="ba-error">{error}</p>}
        </section>
        <section className="ba-note">
          <h3>Discord-first account system</h3>
          <p>Discord login, cloud saves, wallet recovery and signed distribution are not connected in this development build. Local drafts are not authenticated characters. The launcher never silently grants game or wallet access.</p>
        </section>
      </div>
      <footer className="ba-footer"><span>BLACK ANVIL NETWORK</span><span>Launcher + Character Studio · Bevy preview</span></footer>
    </main>
  )
}
