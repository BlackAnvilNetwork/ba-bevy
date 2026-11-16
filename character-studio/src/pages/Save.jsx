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
import { getAtlasSize } from "../library/utils"
import { local } from "../library/store"
import { buildBaCharacterV1, saveBaCharacterDraft } from "../library/ba-character"

const baMode = import.meta.env.VITE_BA_MODE === "1"

function mergeExportOptions() {
  const currentOption = local["mergeOptions_sel_option"] || 0
  const createTextureAtlas =
    local["mergeOptions_create_atlas"] == null
      ? true
      : local["mergeOptions_create_atlas"]
  return {
    createTextureAtlas,
    mToonAtlasSize: getAtlasSize(local["mergeOptions_atlas_mtoon_size"] || 6),
    mToonAtlasSizeTransp: getAtlasSize(
      local["mergeOptions_atlas_mtoon_transp_size"] || 6,
    ),
    stdAtlasSize: getAtlasSize(local["mergeOptions_atlas_std_size"] || 6),
    stdAtlasSizeTransp: getAtlasSize(
      local["mergeOptions_atlas_std_transp_size"] || 6,
    ),
    ktxCompression: local["merge_options_ktx_compression"],
    exportStdAtlas: currentOption === 0 || currentOption == 2,
    exportMtoonAtlas: currentOption === 1 || currentOption == 2,
    twoSidedMaterial: local["mergeOptions_two_sided_mat"] || false,
    optimized: true,
  }
}

function Save() {

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
    try {
      const character = buildBaCharacterV1(characterManager)
      const glbBlob = await characterManager.getGLBBlob(mergeExportOptions())
      const result = await saveBaCharacterDraft(character, glbBlob)
      setDialogMessage(
        `Saved for Black Anvil (rev ${character.revision}). Return to the forge and Enter Greymere.\n${result.json}`,
      )
      setConfirmDialogWindow(true)
      !isMute && playSound("backNextButton")
    } catch (err) {
      console.error(err)
      setDialogMessage(
        `Save failed: ${err?.message || err}. Is Studio running via scripts/run-character-studio.ps1?`,
      )
      setConfirmDialogWindow(true)
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
