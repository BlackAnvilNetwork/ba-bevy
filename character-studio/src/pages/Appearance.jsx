import React, { useContext } from "react"
import styles from "./Appearance.module.css"
import { ViewMode, ViewContext } from "../context/ViewContext"
import { SceneContext } from "../context/SceneContext"
import CustomButton from "../components/custom-button"
import { LanguageContext } from "../context/LanguageContext"
import { SoundContext } from "../context/SoundContext"
import { AudioContext } from "../context/AudioContext"
import FileDropComponent from "../components/FileDropComponent"
import { getFileNameWithoutExtension } from "../library/utils"
import MenuTitle from "../components/MenuTitle"
import BottomDisplayMenu from "../components/BottomDisplayMenu"
import decalPicker from "../images/sticker.png"
import { TokenBox } from "../components/token-box/TokenBox"
import JsonAttributes from "../components/JsonAttributes"
import cancel from "../images/cancel.png"
import DecalGridView from "../components/decals/decalGrid"
import randomizeIcon from "../images/randomize.png"
import colorPicker from "../images/color-palette.png"
import { ChromePicker   } from 'react-color'
import BaColorPicker from "../components/BaColorPicker"
import RightPanel from "../components/RightPanel"
import SaleIcon from "../images/sale-icon.png"

  /**
   * @typedef {import("../library/CharacterManifestData.js").TraitModelsGroup} TraitModelsGroup
   * @typedef {import("../library/CharacterManifestData.js").ModelTrait} ModelTrait
  */

export const TraitPage ={
  TRAIT:0,
  BLEND_SHAPE:1,
  DECAL:2
}

const BA_TRAIT_LABEL = {
  head: "Hair",
  body: "Skin",
  eyes: "Eyes",
  Head: "Hair",
  Body: "Skin",
}

const BA_CUSTOM_TRAITS = new Set(["body", "head", "eyes", "Body", "Head"])

const BA_GENDERS = [
  { id: "male", label: "Male" },
  { id: "female", label: "Female" },
]

/** VRM1 bodies (neuro/drop) — same skeleton as hair. Anata VRM0 broke hair bind. */
const BA_GENDER_MANIFEST = {
  male: {
    url: "./character-assets/neurohacker/manifest.json",
    id: "ba-male",
    name: "Male",
  },
  female: {
    url: "./character-assets/drophunter/manifest.json",
    id: "ba-female",
    name: "Female",
  },
}

function readBaGender() {
  try {
    const fromUrl = new URLSearchParams(window.location.search).get("ba_gender")
    if (fromUrl === "male" || fromUrl === "female") return fromUrl
  } catch (_) { /* ignore */ }
  try {
    const stored = sessionStorage.getItem("ba_gender")
    if (stored === "male" || stored === "female") return stored
  } catch (_) { /* ignore */ }
  return "male"
}

function writeBaGender(gender) {
  try {
    sessionStorage.setItem("ba_gender", gender)
  } catch (_) { /* ignore */ }
  try {
    const url = new URL(window.location.href)
    url.searchParams.set("ba_gender", gender)
    window.history.replaceState({}, "", url.toString())
  } catch (_) { /* ignore */ }
}

const BA_CAM = {
  head: { targetY: 1.45, distance: 1.55 },
  Head: { targetY: 1.45, distance: 1.55 },
  body: { targetY: 0.95, distance: 2.65 },
  Body: { targetY: 0.95, distance: 2.65 },
  eyes: { targetY: 1.48, distance: 1.35 },
  default: { targetY: 0.95, distance: 2.55 },
}

const BA_SKIN_PRESETS = [
  { id: "fair", hex: "#f0c9a8", name: "Fair" },
  { id: "light", hex: "#e0b07a", name: "Light" },
  { id: "medium", hex: "#c48a58", name: "Medium" },
  { id: "tan", hex: "#8d5a38", name: "Tan" },
  { id: "deep", hex: "#5a3824", name: "Deep" },
]

// Sampled from the neurohacker face atlases. The old ramp painted tone1 as a
// light tan and tone4 as deep brown; the textures are the other way around.
const BA_SKIN_SWATCH = {
  originalskin: "#f4cebe",
  tone1skin: "#462728",
  tone2skin: "#463334",
  tone3skin: "#bc7647",
  tone4skin: "#f2cb87",
}

const SKIN_BASE = [0xf4, 0xce, 0xbe]

function hexToRgb(hex) {
  const h = String(hex || "").replace("#", "")
  if (h.length !== 6) return [255, 255, 255]
  return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16) || 0)
}

function skinLitHex(targetHex) {
  const [r, g, b] = hexToRgb(targetHex)
  const lit = [r, g, b].map((channel, i) => Math.round(Math.min(255, (channel / SKIN_BASE[i]) * 255)))
  return `#${lit.map((v) => v.toString(16).padStart(2, "0")).join("")}`
}

function skinLuminance(hex) {
  const [r, g, b] = hexToRgb(hex)
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

function baEyeSwatch(name = "") {
  const n = name.toLowerCase()
  if (n.includes("fire")) return "#e85d04"
  if (n.includes("leaf") || n.includes("kelly") || n.includes("emerald")) return "#4caf50"
  if (n.includes("lime")) return "#c6ff4a"
  if (n.includes("sacramento")) return "#017136"
  if (n.includes("orange")) return "#ff8c1a"
  if (n.includes("amber")) return "#d4a017"
  if (n.includes("yellow")) return "#f5d76e"
  if (n.includes("white")) return "#f2f2f2"
  if (n.includes("wood") || n.includes("latte") || n.includes("clay") || n.includes("chocolate")) return "#8b6914"
  if (n.includes("sky") || n.includes("cyan")) return "#7ec8e3"
  if (n.includes("teal") || n.includes("ocean")) return "#2a9d8f"
  if (n.includes("deep blue") || n.includes("dark blue") || n.includes("navy")) return "#1a3a8a"
  if (n.includes("blue")) return "#4a6cff"
  if (n.includes("lavender") || n.includes("lilac") || n.includes("purple")) return "#9b59b6"
  if (n.includes("scarlet") || n.includes("red")) return "#e53935"
  if (n.includes("pink") || n.includes("light pink")) return "#ff8fab"
  if (n.includes("black")) return "#1a1a1a"
  if (n.includes("dark grey") || n.includes("grey") || n.includes("gray")) return "#8a8f98"
  return "#6e7a8a"
}

function baColorHex(colorTrait) {
  const v = colorTrait?.value
  if (Array.isArray(v) && v.length) return v[0]
  if (typeof v === "string") return v
  return "#888888"
}

function baSkinTextures(selectedTrait) {
  const collection = selectedTrait?.targetTextureCollection?.collection || []
  return collection.filter((t) => /skin$/i.test(t.id))
}

const BA_EYE_TEXTURE_IDS = [
  "blackeyes",
  "chocolateeyes",
  "greyeyes",
  "woodeyes",
  "blueeyes",
  "skyeyes",
  "leafeyes",
  "emeraldeyes",
  "ambereyes",
  "orangeeyes",
  "purpleeyes",
  "lavendereyes",
  "redeyes",
  "pinkeyes",
]

function baEyeTextures(selectedTrait) {
  const all = selectedTrait?.targetTextureCollection?.collection || []
  const preferred = BA_EYE_TEXTURE_IDS
    .map((id) => all.find((t) => t.id === id))
    .filter(Boolean)
  // Fall back to a short slice if ids don't match this pack.
  return preferred.length >= 6 ? preferred : all.slice(0, 12)
}

const BA_HAIR_COLOR_IDS = [
  "darkgrey",
  "copper",
  "latte",
  "clay",
  "maroon",
  "scarlet",
  "red",
  "pink",
  "lavender",
  "violet",
  "blue",
  "navy",
  "emerald",
  "sacramento",
  "kelly",
]

function baHairColors(selectedTrait) {
  const all = selectedTrait?.targetColorCollection?.collection || []
  const preferred = BA_HAIR_COLOR_IDS
    .map((id) => all.find((c) => c.id === id))
    .filter(Boolean)
  return preferred.length ? preferred : all.slice(0, 14)
}

function Appearance() {
  const { isLoading, setViewMode, setIsLoading } = React.useContext(ViewContext)
  const {
    toggleDebugMode,
    characterManager,
    animationManager,
    moveCamera,
  } = React.useContext(SceneContext)
  
  const [traitView, setTraitView] = React.useState(TraitPage.TRAIT)
  const baMode = import.meta.env.VITE_BA_MODE === "1"
  const [traitsVersion, setTraitsVersion] = React.useState(0)

  const { playSound } = React.useContext(SoundContext)
  const { isMute } = React.useContext(AudioContext)
  const { t } = useContext(LanguageContext)

  const visibleGroups = React.useMemo(() => {
    const groups = characterManager.getGroupTraits() || []
    if (!baMode) return groups
    return groups.filter((g) => BA_CUSTOM_TRAITS.has(g.trait))
  }, [characterManager, baMode, traitsVersion])

  React.useEffect(() => {
    setTraitsVersion((v) => v + 1)
    if (baMode) {
      moveCamera({
        targetX: 0.12,
        targetY: BA_CAM.default.targetY,
        targetZ: 0,
        distance: BA_CAM.default.distance,
      })
    }
  }, [])
  

  const back = () => {
    !isMute && playSound('backNextButton');
    characterManager.removeCurrentCharacter();
    characterManager.removeCurrentManifest();
    setViewMode(ViewMode.CREATE)
    toggleDebugMode(false);
  }

  const [jsonSelectionArray, setJsonSelectionArray] = React.useState(null)
  const [traits, setTraits] = React.useState(null)
  /**
  * @type {[TraitModelsGroup, React.Dispatch<TraitModelsGroup>]} state
  */
  const [selectedTraitGroup, setSelectedTraitGroup] = React.useState(null)
  /**
   * @type {[ModelTrait|null, React.Dispatch<ModelTrait|null>]} state
   */
  const [selectedTrait, setSelectedTrait] = React.useState(null)
  const [selectedBlendshapeTraits, setSelectedBlendshapeTraits] = React.useState({})
  const [selectedVRM, setSelectedVRM] = React.useState(null)
  const [loadedAnimationName, setLoadedAnimationName] = React.useState("");
  const [isPickingColor, setIsPickingColor] = React.useState(false)
  const [colorPicked, setColorPicked] = React.useState({ background: '#ffffff' })
  const [baActiveColorId, setBaActiveColorId] = React.useState(null)
  const [baActiveTextureId, setBaActiveTextureId] = React.useState(null)
  const [baCustomActive, setBaCustomActive] = React.useState(false)
  const skinBaseReady = React.useRef(false)
  const [baGender, setBaGender] = React.useState(() => (baMode ? readBaGender() : "male"))
  const [baSide, setBaSide] = React.useState(null) // "gender" | trait id | null

  const next = () => {
    !isMute && playSound('backNextButton');
    setViewMode(ViewMode.SAVE);
    toggleDebugMode(false);
  }

  const randomize = () => {
    setIsLoading(true);
    setJsonSelectionArray(null);
    // BA: only randomize customizable slots — never clothing.
    const p =
      import.meta.env.VITE_BA_MODE === "1"
        ? Promise.all(
            (characterManager.getGroupTraits() || [])
              .map((g) => g.trait)
              .filter((t) => BA_CUSTOM_TRAITS.has(t))
              .map((g) => characterManager.loadRandomTrait(g).catch(() => null)),
          )
        : characterManager.loadRandomTraits()
    p.then(() => {
      if (selectedTraitGroup && selectedTraitGroup.trait != ""){
        setSelectedTrait(characterManager.getCurrentTraitData(selectedTraitGroup.trait));
      }
      setIsLoading(false);
    })
    .catch((error) => {
      setIsLoading(false);
      console.error("Error loading random traits:", error.message);
    });
  }

  const handleColorChange = (color) => {
    setColorPicked({ background: color.hex });
  }
  const handleChangeComplete = (color) =>{
    setColorPicked({ background: color.hex });
    if (baMode && (selectedTraitGroup?.trait === "body" || selectedTraitGroup?.trait === "Body")) {
      applySkinCustom(color.hex)
      return
    }
    characterManager.setTraitColor(selectedTraitGroup?.trait, color.hex);
    if (baMode) {
      setBaCustomActive(true)
      setBaActiveColorId(null)
      setBaActiveTextureId(null)
    }
  }

  const applySkinCustom = async (hex) => {
    const group = selectedTraitGroup?.trait || "body"
    setBaCustomActive(true)
    setBaActiveColorId(null)
    setBaActiveTextureId(null)
    if (!skinBaseReady.current) {
      skinBaseReady.current = true
      const base = baSkinTextures(selectedTrait).find((tex) => tex.id === "originalskin")
      const currentId = characterManager.avatar?.[group]?.textureInfo?.id
      if (base && currentId !== "originalskin") {
        await characterManager._loadTraits([
          { traitModel: selectedTrait, traitTexture: base, traitColor: null },
        ])
      }
    }
    characterManager.setTraitColor(group, skinLitHex(hex))
  }

  const applyBaHairColor = (colorTrait) => {
    const hex = baColorHex(colorTrait)
    setColorPicked({ background: hex })
    setBaActiveColorId(colorTrait.id)
    setBaCustomActive(false)
    setIsPickingColor(false)
    const group = selectedTraitGroup?.trait || "head"
    characterManager.setTraitColor(group, hex)
  }

  const applyBaSkinTint = (hex, id) => {
    setColorPicked({ background: hex })
    setBaActiveColorId(id)
    setBaCustomActive(false)
    setIsPickingColor(false)
    const group = selectedTraitGroup?.trait || "Body"
    characterManager.setTraitColor(group, hex)
  }

  const applyBaTexture = async (textureTrait) => {
    if (!selectedTrait || !selectedTraitGroup) return
    setIsPickingColor(false)
    setBaCustomActive(false)
    setBaActiveTextureId(textureTrait.id)
    skinBaseReady.current = false
    const skinHex = BA_SKIN_SWATCH[textureTrait.id]
    if (skinHex) setColorPicked({ background: skinHex })
    setIsLoading(true)
    try {
      await characterManager._loadTraits([
        {
          traitModel: selectedTrait,
          traitTexture: textureTrait,
          traitColor: null,
        },
      ])
      if (skinHex) {
        try { characterManager.setTraitColor(selectedTraitGroup.trait, "#ffffff") } catch (_) { /* group has no tint target */ }
      }
    } finally {
      setIsLoading(false)
    }
  }

  const openBaCustomColor = () => {
    setBaCustomActive(true)
    setIsPickingColor((v) => !v)
  } 

  const handleAnimationDrop = async (file) => {
    const animName = getFileNameWithoutExtension(file.name);
    const path = URL.createObjectURL(file);
    await animationManager.loadAnimation(path,false,0, true, "", animName);
    setLoadedAnimationName(animationManager.getCurrentAnimationName());
  }

  const handleImageDrop = (file) => {
    setIsPickingColor(false);
    if (selectedTraitGroup && selectedTraitGroup.trait != ""){
      setIsLoading(true);
      const path = URL.createObjectURL(file);
      characterManager.loadCustomTexture(selectedTraitGroup.trait, path).then(()=>{
        setIsLoading(false);
      })
    }
    else{
      console.warn("Please select a group trait first.")
    }
  }
  const handleVRMDrop = (file) =>{
    setIsPickingColor(false);
    if (selectedTraitGroup && selectedTraitGroup.trait != ""){
      setIsLoading(true);
      const path = URL.createObjectURL(file);
      characterManager.loadCustomTrait(selectedTraitGroup.trait, path).then(()=>{
        setIsLoading(false);
      })
    }
    else{
      console.warn("Please select a group trait first.")
    }
  }
  const selectTrait = (trait) => {
    console.log(trait);
    if(trait.id === selectedTrait?.id && trait.collectionID === selectedTrait?.collectionID){
      if(trait.blendshapeTraits?.length>0){
        setTraitView(TraitPage.BLEND_SHAPE);
      }
      // We already selected this trait, do nothing
      return
    }

    setIsPickingColor(false);
    setIsLoading(true);
    const keepHairHex =
      baMode && (trait.traitGroup.trait === "head" || trait.traitGroup.trait === "Head")
        ? colorPicked.background
        : null
    characterManager.loadTrait(trait.traitGroup.trait, trait.id, trait.collectionID).then(()=>{
      setIsLoading(false);
      console.log(characterManager.getCurrentTotalPrice());
      if(trait.blendshapeTraits?.length>0){
        const selectedBlendshapeTrait = characterManager.getCurrentBlendShapeTraitData(trait.traitGroup.trait);
        setSelectedBlendshapeTraits(Object.entries(selectedBlendshapeTrait).reduce((acc,[key,value])=>{acc[key]=value.id;return acc},{}))
        setTraitView(TraitPage.BLEND_SHAPE);
      }
      setSelectedTrait(trait);
      if (keepHairHex) {
        characterManager.setTraitColor(trait.traitGroup.trait, keepHairHex)
      }
    })
  }
  const removeTrait = (traitGroupName) =>{
    setIsPickingColor(false);
    characterManager.removeTrait(traitGroupName);
    setSelectedTrait(null);
  }
  const randomTrait = (traitGroupName) =>{
    setIsPickingColor(false);
    setIsLoading(true);
    characterManager.loadRandomTrait(traitGroupName).then(()=>{
      setIsLoading(false);
      setSelectedTrait(characterManager.getCurrentTraitData(traitGroupName));
    })
    // set selected trait
  }
  const handleJsonDrop = (files) => {
    setIsPickingColor(false);
    const filesArray = Array.from(files);
    const jsonDataArray = [];
    const processFile = (file) => {
      return new Promise((resolve, reject) => {
        if (file && file.name.toLowerCase().endsWith('.json')) {
          const reader = new FileReader();

          // XXX Anata hack to display nft thumbs
          const thumbLocation = `${characterManager.manifestData?.getAssetsDirectory()}/anata/_thumbnails/t_${file.name.split('_')[0]}.jpg`;

          console.log(thumbLocation)
          reader.onload = function (e) {
            try {
              const jsonContent = JSON.parse(e.target.result);
              // XXX Anata hack to display nft thumbs
              jsonContent.thumb = thumbLocation;
              jsonDataArray.push(jsonContent);

              resolve(); // Resolve the promise when processing is complete
            } catch (error) {
              console.error("Error parsing the JSON file:", error);
              reject(error);
            }
          };
          reader.readAsText(file);
        }
      });
    };

    // Use Promise.all to wait for all promises to resolve
    Promise.all(filesArray.map(processFile))
    .then(() => {
      if (jsonDataArray.length > 0){
        // This code will run after all files are processed
        setJsonSelectionArray(jsonDataArray);
        setIsLoading(true);
        characterManager.loadTraitsFromNFTObject(jsonDataArray[0]).then(()=>{
          setIsLoading(false);
        })
      }
    })
    .catch((error) => {
      console.error("Error processing files:", error);
    });
  }

  const handleFilesDrop = async(files) => {
    const file = files[0];
    // Check if the file has the .fbx extension
    if (file && file.name.toLowerCase().endsWith('.fbx')) {
      handleAnimationDrop(file);
    } 
    if (file && (file.name.toLowerCase().endsWith('.png') || file.name.toLowerCase().endsWith('.jpg'))) {
      handleImageDrop(file);
    } 
    if (file && file.name.toLowerCase().endsWith('.vrm')) {
      handleVRMDrop(file);
    } 
    if (file && file.name.toLowerCase().endsWith('.json')) {
      handleJsonDrop(files);
    } 
  };

  const baPackMatchesGender = (gender) => {
    const id = characterManager.manifestDataManager?.mainManifestData?.collectionID
    const pack = BA_GENDER_MANIFEST[gender]
    if (!pack) return false
    if (id === pack.id || id === pack.name) return true
    if (gender === "female" && (id === "Female" || id === "ba-female")) return true
    if (gender === "male" && (id === "Male" || id === "ba-male")) return true
    const dir =
      characterManager.manifestDataManager?.mainManifestData?.traitsDirectory || ""
    if (gender === "female") return /drophunter/i.test(dir)
    if (gender === "male") return /neurohacker/i.test(dir)
    return false
  }

  const selectBaGender = async (gender) => {
    const pack = BA_GENDER_MANIFEST[gender]
    if (!pack) return

    if (gender === baGender && baPackMatchesGender(gender)) {
      !isMute && playSound("optionClick")
      return
    }

    !isMute && playSound("optionClick")
    setBaGender(gender)
    writeBaGender(gender)
    setIsPickingColor(false)
    setBaCustomActive(false)
    setBaActiveColorId(null)
    setBaActiveTextureId(null)
    setTraits(null)
    setSelectedTraitGroup(null)
    setSelectedTrait(null)
    setSelectedBlendshapeTraits({})
    setTraitView(TraitPage.TRAIT)
    setBaSide("gender")
    setIsLoading(true)

    try {
      characterManager.removeCurrentManifest()
      await characterManager.loadManifest(pack.url, pack.id)
      if (animationManager?.setScale) {
        animationManager.setScale(characterManager.manifestDataManager.getDisplayScale())
      }
      await characterManager.loadInitialTraits()
      setTraitsVersion((v) => v + 1)
      moveCamera({
        targetX: 0.12,
        targetY: BA_CAM.default.targetY,
        targetZ: 0,
        distance: BA_CAM.default.distance,
      })
    } catch (err) {
      console.error("Failed to load gender pack:", err)
    } finally {
      setIsLoading(false)
    }
  }

  const openBaGender = () => {
    !isMute && playSound("optionClick")
    setIsPickingColor(false)
    setBaSide("gender")
    setTraits(null)
    setSelectedTraitGroup(null)
    setSelectedTrait(null)
    setSelectedBlendshapeTraits({})
    moveCamera({
      targetX: 0.12,
      targetY: BA_CAM.default.targetY,
      targetZ: 0,
      distance: BA_CAM.default.distance,
    })
  }

  const selectTraitGroup = (traitGroup) => {
    !isMute && playSound('optionClick');
    setIsPickingColor(false);
    if (baMode) setBaSide(traitGroup.trait)
    if (selectedTraitGroup?.trait !== traitGroup.trait){
      setTraitView(TraitPage.TRAIT);
      setTraits(characterManager.getTraits(traitGroup.trait));

      setSelectedTraitGroup(traitGroup);

      const selectedT = characterManager.getCurrentTraitData(traitGroup.trait)
      const selectedBlendshapeTraits = characterManager.getCurrentBlendShapeTraitData(traitGroup.trait);

      setSelectedTrait(selectedT);
      setSelectedBlendshapeTraits(Object.entries(selectedBlendshapeTraits).reduce((acc,[key,value])=>{acc[key]=value.id;return acc},{}))

      setSelectedVRM(characterManager.getCurrentTraitVRM(traitGroup.trait))
      const cam = baMode
        ? (BA_CAM[traitGroup.trait] || BA_CAM.default)
        : { targetY: traitGroup.cameraTarget.height, distance: traitGroup.cameraTarget.distance }
      moveCamera({
        targetX: baMode ? 0.12 : 0,
        targetY: cam.targetY,
        targetZ: 0,
        distance: cam.distance,
      })
      if (baMode) {
        setBaCustomActive(false)
        setBaActiveColorId(null)
        const texId = characterManager.avatar?.[traitGroup.trait]?.textureInfo?.id || null
        setBaActiveTextureId(texId)
        if (traitGroup.trait === "head") {
          setColorPicked({ background: "#c44a3a" })
        } else {
          setColorPicked({ background: "#e8d4b8" })
        }
      }
    }
    else if (!baMode){
      setTraits(null);
      setSelectedTraitGroup(null)
      setSelectedTrait(null);
      setSelectedBlendshapeTraits({})
      setBaActiveColorId(null)
      setBaActiveTextureId(null)
      setBaCustomActive(false)
      moveCamera({
        targetX: 0,
        targetY: 0.8,
        distance: 3.2,
      })
    }
  }


  const uploadTrait = () =>{
    setIsPickingColor(false);
    var input = document.createElement('input');
    input.type = 'file';
    input.accept=".vrm"
    if(!selectedTraitGroup){
      return console.error("Please select a trait group first")
    }
    input.onchange = e => { 
      var file = e.target.files[0]; 
      if (file.name.endsWith(".vrm")){
        const url = URL.createObjectURL(file);
        setIsLoading(true);
        characterManager.loadCustomTrait(selectedTraitGroup.trait,url).then(()=>{
          setIsLoading(false);
        })
      }
    }
    input.click();
  }

  // BA: persist gender; ensure the loaded pack matches selected gender.
  React.useEffect(() => {
    if (!baMode) return
    writeBaGender(baGender)
  }, [baMode, baGender])

  React.useEffect(() => {
    if (!baMode) return
    if (!characterManager.manifestDataManager?.hasExistingManifest?.()) return
    if (baPackMatchesGender(baGender)) return
    selectBaGender(baGender)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [baMode, baGender, traitsVersion])

  React.useEffect(() => {
    if (!baMode || baSide || selectedTraitGroup || !visibleGroups.length) return
    const hair =
      visibleGroups.find((g) => g.trait === "head" || g.trait === "Head") ||
      visibleGroups[0]
    if (hair) selectTraitGroup(hair)
  }, [baMode, visibleGroups, selectedTraitGroup, baSide])

  if (baMode) {
    const traitKey = selectedTraitGroup?.trait
    const hairColors = baHairColors(selectedTrait)
    const skinTextures = baSkinTextures(selectedTrait)
    const eyeTextures = baEyeTextures(selectedTrait)
    const genderOpen = baSide === "gender"

    return (
      <div className={styles.ba}>
        <div className={`loadingIndicator ${isLoading ? "active" : ""}`}>
          <img className={"rotate"} src="ui/loading.svg" />
        </div>

        <div className={styles.baTitle}>
          <div className={styles.baEyebrow}>Black Anvil</div>
          <h1 className={styles.baHeading}>Forge your look</h1>
          <div className={styles.baHint}>
            Hair + peasant tunic &amp; pants
          </div>
        </div>

        <nav className={styles.baCats} aria-label="Appearance categories">
          <button
            type="button"
            className={`${styles.baCat} ${genderOpen ? styles.baCatActive : ""}`}
            onClick={openBaGender}
          >
            <span className={styles.baCatLabel}>Gender</span>
          </button>
          {visibleGroups.map((traitGroup) => {
            const active = !genderOpen && baSide === traitGroup.trait
            return (
              <button
                key={traitGroup.trait}
                type="button"
                className={`${styles.baCat} ${active ? styles.baCatActive : ""}`}
                onClick={() => selectTraitGroup(traitGroup)}
              >
                <span className={styles.baCatLabel}>
                  {BA_TRAIT_LABEL[traitGroup.trait] || traitGroup.name}
                </span>
              </button>
            )
          })}
        </nav>

        {genderOpen && (
          <div className={styles.baDock}>
            <div className={styles.baDockHead}>
              <h2 className={styles.baDockTitle}>Gender</h2>
            </div>
            <div className={styles.baDockBody}>
              <div className={styles.baStyleRow} role="listbox" aria-label="Gender">
                {BA_GENDERS.map((g) => (
                  <button
                    key={g.id}
                    type="button"
                    role="option"
                    aria-selected={baGender === g.id}
                    className={`${styles.baStyleChip} ${baGender === g.id ? styles.baStyleChipActive : ""}`}
                    onClick={() => selectBaGender(g.id)}
                  >
                    {g.label}
                  </button>
                ))}
              </div>
            </div>
          </div>
        )}

        {!!traits && selectedTraitGroup && traitView === TraitPage.TRAIT && !genderOpen && (
          <div className={styles.baDock}>
            <div className={styles.baDockHead}>
              <h2 className={styles.baDockTitle}>
                {BA_TRAIT_LABEL[traitKey] || traitKey}
              </h2>
            </div>

            <div className={styles.baDockBody}>
              {(traitKey === "head" || traitKey === "Head") && (
                <>
                  <div className={styles.baStyleRow} role="listbox" aria-label={traitKey === "Head" ? "Head styles" : "Hair styles"}>
                    {traits.map((trait, index) => {
                      const active =
                        trait.id === selectedTrait?.id &&
                        trait.collectionID === selectedTrait?.collectionID
                      return (
                        <button
                          key={`${trait.id}-${index}`}
                          type="button"
                          role="option"
                          aria-selected={active}
                          className={`${styles.baStyleChip} ${active ? styles.baStyleChipActive : ""}`}
                          onClick={() => selectTrait(trait)}
                        >
                          {trait.name}
                        </button>
                      )
                    })}
                  </div>
                  {(traitKey === "head" || traitKey === "Head") && (
                    <div className={styles.baColorRow} aria-label="Hair colors">
                      {hairColors.map((c) => {
                        const hex = baColorHex(c)
                        const active = !baCustomActive && baActiveColorId === c.id
                        return (
                          <button
                            key={c.id}
                            type="button"
                            title={c.name}
                            className={`${styles.baSwatch} ${active ? styles.baSwatchActive : ""}`}
                            style={{ background: hex }}
                            onClick={() => applyBaHairColor(c)}
                          />
                        )
                      })}
                      <button
                        type="button"
                        title="Custom color"
                        className={`${styles.baSwatch} ${styles.baSwatchCustom} ${baCustomActive ? styles.baSwatchActive : ""}`}
                        onClick={openBaCustomColor}
                      >
                        <span className={styles.baSwatchCustomMark} />
                      </button>
                    </div>
                  )}
                </>
              )}

              {traitKey === "body" && (
                <div className={styles.baColorRow} aria-label="Skin colors">
                  {skinTextures
                    .map((tex) => ({ tex, hex: BA_SKIN_SWATCH[tex.id] || "#c9a07a" }))
                    .sort((a, b) => skinLuminance(b.hex) - skinLuminance(a.hex))
                    .map(({ tex, hex }) => {
                    const currentId = characterManager.avatar?.[traitKey]?.textureInfo?.id
                    const active = !baCustomActive && (baActiveTextureId || currentId) === tex.id
                    return (
                      <button
                        key={tex.id}
                        type="button"
                        title={tex.name}
                        className={`${styles.baSwatch} ${active ? styles.baSwatchActive : ""}`}
                        style={{ background: hex }}
                        onClick={() => applyBaTexture(tex)}
                      />
                    )
                  })}
                  <button
                    type="button"
                    title="Custom color"
                    className={`${styles.baSwatch} ${styles.baSwatchCustom} ${baCustomActive ? styles.baSwatchActive : ""}`}
                    onClick={openBaCustomColor}
                  >
                    <span className={styles.baSwatchCustomMark} />
                  </button>
                </div>
              )}

              {traitKey === "Body" && (
                <div className={styles.baColorRow} aria-label="Skin colors">
                  {BA_SKIN_PRESETS.map((c) => {
                    const active = !baCustomActive && baActiveColorId === c.id
                    return (
                      <button
                        key={c.id}
                        type="button"
                        title={c.name}
                        className={`${styles.baSwatch} ${active ? styles.baSwatchActive : ""}`}
                        style={{ background: c.hex }}
                        onClick={() => applyBaSkinTint(c.hex, c.id)}
                      />
                    )
                  })}
                  <button
                    type="button"
                    title="Custom color"
                    className={`${styles.baSwatch} ${styles.baSwatchCustom} ${baCustomActive ? styles.baSwatchActive : ""}`}
                    onClick={openBaCustomColor}
                  >
                    <span className={styles.baSwatchCustomMark} />
                  </button>
                </div>
              )}

              {traitKey === "eyes" && (
                <div className={styles.baColorRow} aria-label="Eye colors">
                  {eyeTextures.map((tex) => {
                    const hex = baEyeSwatch(tex.name)
                    const active = !baCustomActive && baActiveTextureId === tex.id
                    return (
                      <button
                        key={tex.id}
                        type="button"
                        title={tex.name}
                        className={`${styles.baSwatch} ${active ? styles.baSwatchActive : ""}`}
                        style={{ background: hex }}
                        onClick={() => applyBaTexture(tex)}
                      />
                    )
                  })}
                  <button
                    type="button"
                    title="Custom color"
                    className={`${styles.baSwatch} ${styles.baSwatchCustom} ${baCustomActive ? styles.baSwatchActive : ""}`}
                    onClick={openBaCustomColor}
                  >
                    <span className={styles.baSwatchCustomMark} />
                  </button>
                </div>
              )}
              {!!isPickingColor && (
                <BaColorPicker
                  hex={colorPicked.background}
                  onChange={(hex) => {
                    setColorPicked({ background: hex })
                    handleChangeComplete({ hex })
                  }}
                />
              )}
            </div>
          </div>
        )}

        {traitView === TraitPage.BLEND_SHAPE && (
          <div className={styles.baDock}>
            <BlendShapeTraitView
              selectedTrait={selectedTrait}
              onBack={() => setTraitView(TraitPage.TRAIT)}
              selectedBlendShapeTrait={selectedBlendshapeTraits}
              setSelectedBlendshapeTrait={setSelectedBlendshapeTraits}
            />
          </div>
        )}

        <div className={styles.baFooter}>
          <CustomButton theme="light" text={t("callToAction.back")} size={14} onClick={back} />
          {characterManager.canDownload() && (
            <CustomButton theme="light" text="Done" size={14} onClick={next} />
          )}
        </div>
      </div>
    )
  }

  return (
    <div className={styles.container}>
      <div className={`loadingIndicator ${isLoading ? "active" : ""}`}>
        <img className={"rotate"} src="ui/loading.svg" />
      </div>
      <div className={"sectionTitle"}>{t("pageTitles.chooseAppearance")}</div>
      <FileDropComponent 
         onFilesDrop={handleFilesDrop}
      />
      <div className={styles["sideMenu"]}>
        <MenuTitle title="Appearance" left={20}/>
        <div className={styles["bottomLine"]} />
        <div className={styles["scrollContainer"]}>
          <div className={styles["editor-container"]}>
            {
              characterManager.getGroupTraits().map((traitGroup, index) => (
                <div key={"options_" + index} 
                className={styles["editorButton"]}
                onClick={() => {
                  selectTraitGroup(traitGroup)
                }}>
                  <TokenBox
                    size={56}
                    icon={ traitGroup.fullIconSvg }
                    rarity={selectedTraitGroup?.trait !== traitGroup.trait ? "none" : "mythic"}
                  />
                  <div className={styles["editorText"]}>{traitGroup.name}</div>
                </div>
              ))
            }
          </div>
        </div>
      </div>

      {!!traits && selectedTraitGroup && (
        <div className={styles["selectorContainerPos"]}>
          <MenuTitle title={selectedTraitGroup.trait} width={130} left={20}/>
          <div className={styles["selectorPickerTabs"]}>
              {selectedTrait && traitView==TraitPage.TRAIT && (
                <div className={styles["selectorColorPickerButton"]}
                  onClick={()=>{setIsPickingColor(!isPickingColor)}}
                  >
                  <img className={styles["selectorColorPickerImg"]} src={colorPicker}/>
                </div>
                 )}
                {selectedTraitGroup && selectedTraitGroup.getAllDecals()?.length && <div className={styles["selectorColorPickerButton"]}
                  onClick={()=>traitView==TraitPage.DECAL?setTraitView(TraitPage.TRAIT):setTraitView(TraitPage.DECAL)}
                  >
                  <img className={styles["selectorColorPickerImg"]} src={decalPicker}/>
                </div>}
            </div>
            {
          traitView==TraitPage.TRAIT && !!isPickingColor && (<div 
            draggable = {false}
            className={styles["selectorColorPickerUI"]}>
            <ChromePicker 
              styles={{ default: {picker:{ width: '200px' }} }}
              color={ colorPicked.background }
              onChange={ handleColorChange }
              onChangeComplete={ handleChangeComplete }
              />
          </div>)}
          <div className={styles["bottomLine"]} />
          <div className={styles["scrollContainerOptions"]}>
          {traitView == TraitPage.TRAIT && (
            <div className={styles["selector-container"]}>
              {
                <div
                  key={"randomize-trait"}
                  className={`${styles["selectorButton"]}`}
                  onClick={() => {randomTrait(selectedTraitGroup.trait)}}
                >
                  <TokenBox
                    size={56}
                    icon={randomizeIcon}
                    rarity={"none"}
                  />
                </div>
              }
                {!characterManager.isTraitGroupRequired(selectedTraitGroup.trait) ? (
                  <div
                    key={"no-trait"}
                    className={`${styles["selectorButton"]}`}
                    icon={cancel}
                    onClick={() => {removeTrait(selectedTraitGroup.trait)}}
                  >
                    <TokenBox
                      size={56}
                      icon={cancel}
                      rarity={selectedTrait == null ? "mythic" : "none"}
                    />
                  </div>
                ) : (
                  <></>
                )
              }
              {traits.map((trait, index) => {
                let active = (trait.id === selectedTrait?.id && trait.collectionID === selectedTrait?.collectionID)
                return (
                  <div
                    key={index}
                    className={`${styles["selectorButton"]}`}
                    onClick={()=>{selectTrait(trait)}}
                  >
                    <TokenBox
                      size={56}
                      iconOverlay={(trait.purchasable && trait.locked) ? SaleIcon:null}
                      icon={trait.fullThumbnail}
                      rarity={active ? "mythic" : "none"}      
                    />
                  </div>
                )
              })}
            </div>)}
            {traitView == TraitPage.BLEND_SHAPE && (
              <BlendShapeTraitView selectedTrait={selectedTrait} onBack={()=>{setTraitView(TraitPage.TRAIT)}} selectedBlendShapeTrait={selectedBlendshapeTraits} setSelectedBlendshapeTrait={setSelectedBlendshapeTraits} />
            )}
            {traitView == TraitPage.DECAL && (
              <DecalGridView selectedTraitGroup={selectedTraitGroup} onBack={()=>{setTraitView(TraitPage.TRAIT)}} />
            )}
          </div>
          
          <div className={styles["uploadContainer"]}>
            <div 
              className={styles["uploadButton"]}
              onClick={uploadTrait}>
              <div> 
                Upload </div>
            </div>
            
          </div>
        </div>
      )}
      <JsonAttributes jsonSelectionArray={jsonSelectionArray}/>
      
      <RightPanel selectedTrait={selectedTrait} selectedVRM={selectedVRM} traitGroupName={selectedTraitGroup?.trait||""}/>

      <BottomDisplayMenu loadedAnimationName={loadedAnimationName} randomize={randomize}/>
      <div className={styles.buttonContainer}>
        <CustomButton
          theme="light"
          text={t('callToAction.back')}
          size={14}
          className={styles.buttonLeft}
          onClick={back}
        />

        {
        characterManager.canDownload() &&
          <CustomButton
            theme="light"
            text={t('callToAction.next')}
            size={14}
            className={styles.buttonRight}
            onClick={next}
          />
        }
      </div>
    </div>
  )
}

export default Appearance

/**
 * @param {{selectedTrait:ModelTrait|null,selectedBlendShapeTrait:Record<string,string>,onBack:()=>void,setSelectedBlendshapeTrait:(x:Record<string,string>)=>void}} param0 
 */
const BlendShapeTraitView = ({selectedTrait,onBack,selectedBlendShapeTrait,setSelectedBlendshapeTrait})=>{
  const {characterManager,moveCamera} = React.useContext(SceneContext);

  const groups = characterManager.getBlendShapeGroupTraits(selectedTrait?.traitGroup.trait||"",selectedTrait?.id||"");

  /**
   *
   * @param {string} traitGroup
   * @param {import('../library/CharacterManifestData').BlendShapeGroup} blendShapeGroupTrait 
    */
  const removeBlendShapeTrait = (traitGroup,blendShapeGroupTrait)=>{
    characterManager.removeBlendShapeTrait(traitGroup,blendShapeGroupTrait.trait);
    const blendShapeTraitCopy = {...selectedBlendShapeTrait};
    delete blendShapeTraitCopy[blendShapeGroupTrait.trait]
    setSelectedBlendshapeTrait(blendShapeTraitCopy);
  }
  /**
   * @param {import('../library/CharacterManifestData').BlendShapeTrait} newBlendShape 
   */
  const selectBlendShapeTrait = (newBlendShape)=>{
    const parent = newBlendShape.parentGroup;
    characterManager.loadBlendShapeTrait(selectedTrait?.traitGroup.trait||"",parent.trait||"",newBlendShape?.id||'');
    moveCamera({ targetY: parent.cameraTarget.height, distance: parent.cameraTarget.distance})
    const blendShapeTraitCopy = {...selectedBlendShapeTrait};
    blendShapeTraitCopy[parent.trait||''] = newBlendShape.id;
    setSelectedBlendshapeTrait(blendShapeTraitCopy);
  }

  return (
    <div className={styles["selector-container-column"]}>
        <CustomButton
          theme="dark"
          text={"Back"}
          size={14}
          className={styles.buttonLeft}
          onClick={onBack}
        />
        {groups && groups.length > 0 && groups.map((group)=>{
          return (
            <div key={group.trait} className={styles.blendshapeGroup}> 
              <div>{group.name}</div>
              <div className={styles["selector-container"]} >
                <BlendShapeItem key={"empty"}
                    src={cancel}
                    active={!selectedBlendShapeTrait[group.trait]}
                    blendshapeID="cancel"
                    select={()=>removeBlendShapeTrait(selectedTrait.traitGroup.trait,group)}
                    />
                {group.collection.map((blendShapeTrait)=>{
                  let active = blendShapeTrait.id === selectedBlendShapeTrait[group.trait]
                  return (
                    <BlendShapeItem key={blendShapeTrait.id} src={blendShapeTrait.fullThumbnail||''} active={active} blendshapeID={blendShapeTrait.id} select={()=>selectBlendShapeTrait(blendShapeTrait)}/>
                  )
                })}
              </div>

            </div>
          )
        })}
    </div>
  )
}

/**
 * @param {{active:boolean,blendshapeID:string,src:string,select:()=>void}} param0 
 */
const BlendShapeItem = ({active,blendshapeID,src,select})=>{

  return (
    <div
      key={blendshapeID}
      className={`${styles["selectorButton"]}`}
      onClick={select}
    >
      <TokenBox
        size={56}
        icon={src||''}
        rarity={active ? "mythic" : "none"}      
      />
    </div>
  )
}