import React, { useEffect, useState } from "react"
import styles from "./Create.module.css"
import { ViewMode, ViewContext } from "../context/ViewContext"
import CustomButton from "../components/custom-button"
import { LanguageContext } from "../context/LanguageContext"
import { useContext } from "react"

import { SceneContext } from "../context/SceneContext"
import { SoundContext } from "../context/SoundContext"
import { AudioContext } from "../context/AudioContext"

import { getAsArray } from "../library/utils"

function Create() {
  
  // Translate hook
  const {t} = useContext(LanguageContext);

  const { setViewMode, setIsLoading, isLoading } = React.useContext(ViewContext)
  const { playSound } = React.useContext(SoundContext)
  const { isMute } = React.useContext(AudioContext)
  const { manifest, characterManager } = React.useContext(SceneContext)
  const [ classes, setClasses ] = useState([])
  const [creationError, setCreationError] = useState("")
  // Prevent StrictMode/effect re-renders from loading the same pack twice.
  const initializationStarted = React.useRef(false)


  const baMode = import.meta.env.VITE_BA_MODE === "1"

  useEffect(() => {

    if (manifest?.characters != null){
      const manifestClasses = getCharacterManifests(getAsArray(manifest.characters));
      setClasses(manifestClasses);
    }
  }, [manifest])

  // BA: default gender is male unless ?ba_gender= says otherwise.
  useEffect(() => {
    if (!baMode) return
    try {
      const fromUrl = new URLSearchParams(window.location.search).get("ba_gender")
      const gender = fromUrl === "female" ? "female" : "male"
      sessionStorage.setItem("ba_gender", gender)
      if (fromUrl !== "male" && fromUrl !== "female") {
        const url = new URL(window.location.href)
        url.searchParams.set("ba_gender", "male")
        window.history.replaceState({}, "", url.toString())
      }
    } catch (_) { /* ignore */ }
  }, [baMode])

  const back = () => {
    setViewMode(ViewMode.LANDING)
    !isMute && playSound('backNextButton');
  }

  const getCharacterManifests = (charactersArray) =>{
      return charactersArray.map((c) => {
        return {
          name:c.name, 
          portrait:c.portrait, 
          description: c.description,
          manifest: c.manifest,
          icon:c.icon,
          format:c.format,
          baGender: c.baGender,
          manifestAppend: getCharacterManifests(getAsArray(c.manifestAppend)),
        }
      })
  }

  const selectClass = async (index) => {
    if (initializationStarted.current) return
    const selectedClass = classes[index]
    if (!selectedClass || !characterManager) {
      setCreationError("Character assets are not ready. Please try again.")
      return
    }
    initializationStarted.current = true
    setCreationError("")
    setIsLoading(true)

    try {
      const gender =
        selectedClass.baGender === "female" || selectedClass.baGender === "male"
          ? selectedClass.baGender
          : selectedClass.name?.toLowerCase() === "female"
            ? "female"
            : "male"
      try {
        sessionStorage.setItem("ba_gender", gender)
      } catch (_) { /* storage can be disabled */ }

      const manifestId = gender === "female" ? "ba-female" : "ba-male"
      await characterManager.loadManifest(
        selectedClass.manifest,
        baMode ? manifestId : selectedClass.name,
      )

      // Do not open the appearance editor before the model exists.
      await Promise.all(
        selectedClass.manifestAppend.map((append) =>
          characterManager.loadManifest(append.manifest, append.name),
        ),
      )
      await characterManager.loadInitialTraits()

      if (baMode && !Object.values(characterManager.avatar || {}).some((trait) => trait?.vrm)) {
        throw new Error("The character model did not finish loading.")
      }

      setViewMode(ViewMode.APPEARANCE)
      !isMute && playSound("classSelect")
    } catch (error) {
      console.error("Failed to initialize character:", error)
      setCreationError(error?.message || "Unable to load character. Please retry.")
    } finally {
      initializationStarted.current = false
      setIsLoading(false)
    }
  }

  // BA: skip class grid — jump straight into the pack matching ba_gender (default male).
  useEffect(() => {
    if (!baMode || !classes.length || !characterManager || isLoading || creationError ||
        initializationStarted.current) return
    // A returning player may already have a fully loaded model from an earlier
    // editor visit. Reuse it, but never treat a manifest alone as an avatar.
    if (characterManager.manifestDataManager?.hasExistingManifest?.() &&
        Object.values(characterManager.avatar || {}).some((trait) => trait?.vrm)) {
      setViewMode(ViewMode.APPEARANCE)
      return
    }
    let gender = "male"
    try {
      const fromUrl = new URLSearchParams(window.location.search).get("ba_gender")
      const stored = sessionStorage.getItem("ba_gender")
      if (fromUrl === "female" || fromUrl === "male") gender = fromUrl
      else if (stored === "female" || stored === "male") gender = stored
    } catch (_) { /* ignore */ }
    const idx = classes.findIndex(
      (c) => c.baGender === gender || c.name?.toLowerCase() === gender,
    )
    selectClass(idx >= 0 ? idx : 0)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [baMode, classes, characterManager, isLoading, creationError])

  const hoverSound = () => {
    !isMute && playSound('classMouseOver');
  }
  
  if (baMode) {
    return (
      <div className={styles.baLoading} role="status" aria-live="polite">
        <h2>{creationError ? "Character loading failed" : "Forging your character…"}</h2>
        <p>{creationError || "Preparing your character model and customization options."}</p>
        {creationError && (
          <button
            type="button"
            onClick={() => {
              setCreationError("")
              const index = classes.findIndex((c) => c.baGender === "female" &&
                sessionStorage.getItem("ba_gender") === "female")
              selectClass(index >= 0 ? index : 0)
            }}
          >
            Retry character loading
          </button>
        )}
      </div>
    )
  }

  return (
    <div className={`${styles.container} horizontalScroll`}>
      <div className={"sectionTitle"}>{t('pageTitles.chooseClass')}</div>
      <div className={styles.vrmOptimizerButton}>
      </div>

      
      <div className={styles.topLine} />
      
      <div className={styles.classContainer}>
        {classes.map((characterClass, i) => {
          return (
            <div
              key={i}
              className={
                !characterClass["disabled"]
                  ? styles.class
                  : styles.classdisabled
              }
              onClick={
                  () => selectClass(i)
              }
              onMouseOver={
                  () => hoverSound()
              }
            >
            <div
                className={styles.classFrame}
                style={{
                  "backgroundImage": `url(${characterClass["portrait"]})`,
                }}
              >
                <div className={styles.frameContainer}>
                  <img
                    src={"./assets/backgrounds/class-frame.svg"}
                    className={styles.frame}
                  />
                </div>

              </div>
              
              <div className={styles.name}>{characterClass["name"]}</div>
              <div className={styles.description}>
                {characterClass["description"]}
              </div>
            </div>
          )
        })}
      </div>

      <div className={styles.bottomLine} />
      <div className={styles.buttonContainer}>
        { <CustomButton
          theme="light"
          text={t('callToAction.back')}
          size={14}
          className={styles.buttonLeft}
          onClick={back}
      />}
      </div>
    </div>
  )
}

export default Create
