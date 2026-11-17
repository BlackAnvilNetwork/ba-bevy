import React, { useContext, useState } from "react"
import styles from "./Save.module.css"
import { ExportMenu } from "../components/ExportMenu"
import { SceneContext } from "../context/SceneContext"
import { ViewMode, ViewContext } from "../context/ViewContext"
import CustomButton from "../components/custom-button"
import { LanguageContext } from "../context/LanguageContext"
import { SoundContext } from "../context/SoundContext"
import { AudioContext } from "../context/AudioContext"
import MessageWindow from "../components/MessageWindow"
import MergeOptions from "../components/MergeOptions"
import FileDropComponent from "../components/FileDropComponent"
import PurchaseMenu from "../components/PurchaseMenu"
import { finishBaCharacterForGame } from "../library/ba-game-handoff"

const baMode = import.meta.env.VITE_BA_MODE === "1"

function Save({ onCharacterSaved }) {

  // Translate hook
  const { t } = useContext(LanguageContext);
  const { playSound } = React.useContext(SoundContext)
  const { isMute } = React.useContext(AudioContext)
  const { setViewMode } = React.useContext(ViewContext);
  const { characterManager } = React.useContext(SceneContext)


  const [confirmDialogWindow, setConfirmDialogWindow] = useState(false)
  const [dialogMessage, setDialogMessage] = useState("")
  const [baSaving, setBaSaving] = useState(false)

  const [currentPrice, setCurrentPrice] = React.useState(0)
  const [purchaseTraits, setPurchaseTraits] = React.useState([])
  const [currency, setCurrency] = React.useState("")

  React.useEffect(() => {
    setCurrentPrice(characterManager.getCurrentTotalPrice());
    setCurrency(characterManager.getMainPriceCurrency());
  }, [])

  const back = () => {
    setViewMode(ViewMode.APPEARANCE)
    !isMute && playSound('backNextButton');
  }
  const mint = () => {
    setViewMode(ViewMode.MINT)
    !isMute && playSound('backNextButton');
  }
  const saveForBlackAnvil = async () => {
    if (baSaving) return
    setBaSaving(true)
    setDialogMessage("")
    try {
      const { character } = await finishBaCharacterForGame(characterManager)
      if (typeof onCharacterSaved === "function") {
        onCharacterSaved(character)
      } else {
        setDialogMessage("Character saved. Open the Black Anvil launcher to play.")
      }
      !isMute && playSound("backNextButton")
    } catch (err) {
      console.error("Black Anvil save failed:", err)
      setDialogMessage(`Save failed: ${err?.message || err}. Your edits are still here; please retry.`)
    } finally {
      setBaSaving(false)
    }
  }
  const onPurchaseClick = async() =>{
    //console.log(characterManager.getPurchaseTraitsArray());
    console.log("click purch")
    setPurchaseTraits(characterManager.getPurchaseTraitsArray())
  }
  const handleFilesDrop = async(files) => {
    const file = files[0];
    if (file && file.name.toLowerCase().endsWith('.json')) {
    } 
  };
  const onConfirmPurchase = () =>{
    console.log("confirm purchase!!")
    characterManager.purchaseAssetsFromAvatar()
      .then(()=>{
        setCurrentPrice(characterManager.getCurrentTotalPrice());
        setPurchaseTraits([]);
        setConfirmDialogWindow(true);
        setDialogMessage("Purchase successful");
      })
      .catch((e)=>{
        setConfirmDialogWindow(true);
        setDialogMessage("An error occurred when trying to purchase assets. Please try again.");
      })
  }
  const cancelPurchase = () =>{
    setPurchaseTraits([]);
  }

  // Black Anvil always saves the playable character; never expose the
  // standalone Studio Download Options or ExportMenu in launcher mode.
  if (baMode) {
    return (
      <div className={styles.container}>
        <h2>Finish your character</h2>
        <p>Your customized avatar will be saved to Black Anvil automatically.</p>
        <div style={{ display: "flex", gap: 16 }}>
          <button type="button" disabled={baSaving} onClick={back}>
            Continue editing
          </button>
          <button type="button" disabled={baSaving} onClick={saveForBlackAnvil}>
            {baSaving ? "Saving character…" : "Save and return to launcher"}
          </button>
        </div>
        {dialogMessage && <p role="status">{dialogMessage}</p>}
      </div>
    )
  }

  return (
    <div className={styles.container}>
      
      <div className={"sectionTitle"}>{t("pageTitles.saveCharacter")}</div>
      <div className={styles.buttonContainer}>
        <FileDropComponent 
          onFilesDrop={handleFilesDrop}
        />
        <CustomButton
          theme="light"
          text={t('callToAction.back')}
          size={14}
          className={styles.buttonLeft}
          onClick={back}
        />
        {purchaseTraits.length > 0 && (
          <PurchaseMenu
            currentPrice = {currentPrice}
            purchaseTraits = {purchaseTraits}
            onConfirmPurchase = {onConfirmPurchase}
            cancelPurchase = {cancelPurchase}
            currency = {currency}
            
        />)}
        
        <MergeOptions
          showCreateAtlas = {true}
          mergeMenuTitle = {"Download Options"}
        />
        <ExportMenu 
          currentPrice = {currentPrice}
          onPurchaseClick = {onPurchaseClick}
        />

        {baMode && (
          <CustomButton
            theme="light"
            text={baSaving ? "Saving…" : "Save for Black Anvil"}
            size={14}
            className={styles.buttonRight}
            onClick={saveForBlackAnvil}
          />
        )}
        
        {!baMode && (
          <CustomButton
              theme="light"
              text="mint"
              size={14}
              className={styles.buttonRight}
              onClick={mint}
          />
        )}
      </div>
      <MessageWindow
        cancelOption = {false}
        confirmDialogText = {dialogMessage}
        confirmDialogCallback = {[]}
        confirmDialogWindow = {confirmDialogWindow}
        setConfirmDialogWindow = {setConfirmDialogWindow}
      />
    </div>
  )
}

export default Save
