import React, { Fragment, useContext, useEffect, useState } from "react"
import * as THREE from "three"

import { LanguageContext } from "./context/LanguageContext"
import { SceneContext } from "./context/SceneContext"
import { ViewContext, ViewMode } from "./context/ViewContext"
import { EffectManager } from "./library/effectManager"
//import { AnimationManager } from "./library/animationManager"
import MessageWindow from "./components/MessageWindow"
import BaLauncherDashboard from "./components/BaLauncherDashboard"

import Background from "./components/Background"

import Appearance from "./pages/Appearance"
import BatchDownload from "./pages/BatchDownload"
import BatchManifest from "./pages/BatchManifest"
import Claim from "./pages/Claim"
import Create from "./pages/Create"
import Landing from "./pages/Landing"
import Load from "./pages/Load"
import Mint from "./pages/Mint"
import Optimizer from "./pages/Optimizer"
import Save from "./pages/Save"
import Wallet from "./pages/Wallet"
import { ingestBaForgeQuery } from "./library/ba-character"
import { hasLoadedAvatar } from "./library/ba-creator"

// dynamically import the manifest
const assetImportPath = (import.meta.env.VITE_ASSET_PATH || "") + "/manifest.json"
//const assetImportPath = "./manifest.json"

const baMode = import.meta.env.VITE_BA_MODE === "1"
if (baMode) {
  ingestBaForgeQuery()
}
const cameraDistanceOther = baMode ? 2.55 : 6
const centerCameraTargetOther = new THREE.Vector3(baMode ? 0.12 : 0, baMode ? 0.95 : 0.8, 0)
const centerCameraPositionOther = new THREE.Vector3(
  -2.2367993753934425,
  1.1512971720174363,
  2.2612065299409223,
) // note: get from `moveCamera({ targetY: 0.8, distance: 3.2 })`
async function fetchManifest(location) {
  try {
    const response = await fetch(location);
    
    if (!response.ok) {
      throw new Error(`Failed to fetch manifest. Status: ${response.status}`);
    }
    
    const data = await response.json();
    return data;
  } catch (error) {
    console.error(`Error fetching manifest: ${error.message}`);
    return [];
  }
}


async function fetchAll() {
  const initialManifest = await fetchManifest(assetImportPath)
  const effectManager = new EffectManager()

  return {
    initialManifest,
    effectManager,
  }
}

const fetchData = () => {
  let status, result

  const manifestPromise = fetchAll()
  // const modelPromise = fetchModel()
  const suspender = manifestPromise.then(
    (r) => {
      status = "success"
      result = r
    },
    (e) => {
      status = "error"
      result = e
    },
  )

  return {
    read() {
      if (status === "error") {
        throw result
      } else if (status === "success") {
        return result
      }
      throw suspender
    },
  }
}

const resource = fetchData()

export default function App() {
  const {
    initialManifest,
    effectManager,
  } = resource.read()
  const [hideUi, setHideUi] = useState(false)
  // The Tauri window is the launcher. Character Studio is an in-app view.
  const [launcherHome, setLauncherHome] = useState(baMode && Boolean(window.__TAURI__))
  const {
    camera,
    controls,
    scene,
    moveCamera,
    setManifest,
    lookAtManager,
    showEnvironmentModels,
    characterManager,
  } = useContext(SceneContext)
  const { viewMode, setViewMode } = useContext(ViewContext)

  effectManager.camera = camera
  effectManager.scene = scene

  const updateCameraPosition = () => {
    if (!effectManager.camera) return

      moveCamera({
        // center
        targetX: 0,
        targetY: centerCameraTargetOther.y,
        targetZ: 0,
        distance: cameraDistanceOther,
      })

    if (controls) {
      if (
        [ViewMode.APPEARANCE, ViewMode.SAVE, ViewMode.OPTIMIZER, ViewMode.BATCHDOWNLOAD, ViewMode.BATCHMANIFEST].includes(viewMode)
      ) {
        controls.enabled = true
      } else {
        controls.enabled = false
      }
    }
  }

  const [confirmDialogWindow, setConfirmDialogWindow] = useState(false)
  const [confirmDialogText, setConfirmDialogText] = useState("")
  const [confirmDialogCallback, setConfirmDialogCallback] = useState([])

  const confirmDialog = (msg, callback) => {
    setConfirmDialogText(msg)
    setConfirmDialogWindow(true)
    setConfirmDialogCallback([callback])
  }

  // map current app mode to a page
  const pages = {
    [ViewMode.LANDING]: <Landing />,
    [ViewMode.APPEARANCE]: (
      <Appearance
        confirmDialog={confirmDialog}
        onCharacterSaved={baMode && window.__TAURI__ ? () => {
          // The launcher remounts and reads the newly selected native revision.
          // Play remains an explicit player action after the save succeeds.
          setLauncherHome(true)
        } : undefined}
      />
    ),
    [ViewMode.OPTIMIZER]:<Optimizer/>,
    [ViewMode.CREATE]: <Create />,
    [ViewMode.WALLET]: <Wallet />,
    [ViewMode.CLAIM]: <Claim />,
    [ViewMode.BATCHMANIFEST]: <BatchManifest />,
    [ViewMode.BATCHDOWNLOAD]: <BatchDownload />,
    [ViewMode.LOAD]: <Load />,
    [ViewMode.MINT]: <Mint />,
    [ViewMode.SAVE]: <Save onCharacterSaved={baMode && window.__TAURI__ ? () => setLauncherHome(true) : undefined} />,
  }

  let lastTap = 0
  useEffect(() => {
    const handleTap = (e) => {
      const now = new Date().getTime()
      const timesince = now - lastTap
      if (timesince < 300 && timesince > 10) {
        const tgt = e.target
        if (tgt.id == "editor-scene") setHideUi(!hideUi)
      }
      lastTap = now
    }
    window.addEventListener("touchend", handleTap)
    window.addEventListener("click", handleTap)
    return () => {
      window.removeEventListener("touchend", handleTap)
      window.removeEventListener("click", handleTap)
    }
  }, [hideUi])

  useEffect(() => {
    if (lookAtManager != null){
      updateCameraPosition()
      lookAtManager.enabled = true
      if ([ViewMode.LANDING, ViewMode.CREATE, ViewMode.CLAIM, ViewMode.LOAD, ViewMode.CLAIM, ViewMode.CLAIM].includes(viewMode))
        showEnvironmentModels(false)
      else
        showEnvironmentModels(true)
      window.addEventListener("resize", updateCameraPosition)
      return () => {
        window.removeEventListener("resize", updateCameraPosition)
      }
    }
    

  }, [viewMode, lookAtManager])

  useEffect(() => {
    setManifest(initialManifest)
  }, [initialManifest])

  // Translate hook
  const {t} = useContext(LanguageContext);

  if (launcherHome) {
    return <BaLauncherDashboard onCustomize={() => {
      // APPEARANCE only edits an avatar that is already in the Three.js scene.
      // On first entry the CREATE page must load the selected gender's pack
      // and initial traits before opening APPEARANCE. Skipping that step
      // produced a blank character until the player pressed Back.
      setViewMode(hasLoadedAvatar(characterManager) ? ViewMode.APPEARANCE : ViewMode.CREATE)
      setLauncherHome(false)
    }} />
  }

  return (
    <Fragment>
      {baMode && window.__TAURI__ && (
        <button type="button" className="ba-back-to-launcher" onClick={() => setLauncherHome(true)}>
          ← Back to launcher
        </button>
      )}
      
      <div className="generalTitle">{baMode ? "Black Anvil" : "Character Studio"}</div>

      {/* <LanguageSwitch /> */}
      <MessageWindow
        confirmDialogText = {confirmDialogText}
        confirmDialogCallback = {confirmDialogCallback}
        confirmDialogWindow = {confirmDialogWindow}
        setConfirmDialogWindow = {setConfirmDialogWindow}
      />
      <Background />
      
      {pages[viewMode]}
      
    </Fragment>
  )
}
