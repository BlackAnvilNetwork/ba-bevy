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
  const [account, setAccount] = useState(null)
  const [authPending, setAuthPending] = useState(false)
  const [authMessage, setAuthMessage] = useState("Discord sign-in required for online play")

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

  useEffect(() => {
    if (typeof invoke !== "function") return
    let active = true
    invoke("discord_account").then((value) => {
      if (active && value?.connected) {
        setAccount(value)
        setAuthMessage("Connected to Discord")
      }
    }).catch((e) => {
      if (active) setAuthMessage(String(e?.message || e))
    })
    return () => { active = false }
  }, [invoke])

  useEffect(() => {
    if (!authPending || typeof invoke !== "function") return
    let active = true
    const timer = setInterval(async () => {
      try {
        const outcome = await invoke("poll_discord_login")
        if (!active) return
        if (outcome?.status === "complete") {
          setAuthPending(false)
          setAccount({ connected: true, accountId: outcome.accountId, displayName: outcome.displayName })
          setAuthMessage("Discord connected — multiplayer ready")
        }
      } catch (e) {
        if (active) {
          setAuthPending(false)
          setAuthMessage(String(e?.message || e))
        }
      }
    }, 1400)
    return () => { active = false; clearInterval(timer) }
  }, [authPending, invoke])

  const signIn = async () => {
    setError("")
    try {
      const result = await invoke("begin_discord_login")
      const open = window.__TAURI__?.shell?.open
      if (typeof open !== "function") throw new Error("System browser opening is unavailable")
      await open(result.authorizationUrl)
      setAuthPending(true)
      setAuthMessage("Approve Black Anvil in Discord; this launcher will update automatically.")
    } catch (e) {
      setAuthMessage(String(e?.message || e))
    }
  }

  const signOut = async () => {
    try {
      await invoke("discord_logout")
      setAccount(null)
      setMultiplayer(false)
      setAuthMessage("Discord disconnected")
    } catch (e) {
      setError(String(e?.message || e))
    }
  }

  const play = async () => {
    if (busy || !character || typeof invoke !== "function") return
    if (multiplayer && !account?.connected) {
      setError("Sign in with Discord before entering the online world.")
      return
    }
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
        <section className="ba-auth-panel" aria-label="Discord account">
          <div>
            <strong>{account?.connected ? `Discord · ${account.displayName}` : "Sign in with Discord"}</strong>
            <p>{authMessage}</p>
          </div>
          {account?.connected
            ? <button className="ba-secondary" type="button" onClick={signOut}>Sign out</button>
            : <button className="ba-primary" type="button" disabled={authPending} onClick={signIn}>
                {authPending ? "Connecting…" : "Connect Discord"}
              </button>}
        </section>
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
          <label className="ba-multi"><input type="checkbox" checked={multiplayer} disabled={!account?.connected} onChange={(e) => setMultiplayer(e.target.checked)} /> Multiplayer — authenticated Discord character and one-use zone ticket</label>
          {error && <p role="alert" className="ba-error">{error}</p>}
        </section>
        <section className="ba-note">
          <h3>Account-owned multiplayer</h3>
          <p>Online Play synchronizes your verified character to the Black Anvil gateway and issues a single-use zone ticket. Local preview remains available without a network. Wallet keys, NFT minting, packaged game updates and Discord party voice/text are separate features.</p>
        </section>
      </div>
      <footer className="ba-footer"><span>BLACK ANVIL NETWORK</span><span>Launcher + Character Studio · Bevy preview</span></footer>
    </main>
  )
}
