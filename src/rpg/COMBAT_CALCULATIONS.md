# Black Anvil — Combat System Explicit Calculations

Source of truth for combat math. Runtime ports live under `src/rpg/` and must converge on these formulas. Where a note says a simplified model is incomplete, keep the note and implement the piecewise form before treating it as exact.

Derived from classical FFXI-style combat math; adapted for Black Anvil. Implementation must identify which retail / private-server revision applies before treating historical pieces as current.

---

## 1. Physical Damage

### Single melee hit

\[
D=(WDMG+fSTR)\times pDIF
\]

### Single ranged hit

\[
D=(WDMG+AMG+fSTR2)\times pDIF
\]

### Melee fSTR

\[
fSTR=\left\lfloor\frac{STR-VIT+4}{4}\right\rfloor
\]

### Ranged fSTR2

\[
fSTR2=\left\lfloor\frac{STR-VIT+4}{2}\right\rfloor
\]

### Attack ratio

\[
cRatio=\frac{ATK}{DEF}-0.05(dLVL)
\]

### Maximum pDIF (simplified linear)

\[
pDIF_{max}=0.4+1.2(cRatio)
\]

### Minimum pDIF (simplified linear)

\[
pDIF_{min}=-0.5+1.2(cRatio)
\]

\[
pDIF=\operatorname{Random}(pDIF_{min},pDIF_{max})
\]

\[
pDIF_{min}\geq0
\]

See §15 for piecewise weapon-category pDIF. Prefer §15 over the linear model for production.

### Physical weapon skill

\[
D=(WDMG+fSTR+WSC)\times fTP\times pDIF
\]

### Weapon skill stat modifier

\[
WSC=\left\lfloor STAT_1(A)+STAT_2(B)+\cdots\right\rfloor
\]

See §12 for level-adjusted WSC (\(\alpha\)).

### Physical damage taken (monster → player)

\[
D=(MonsterDMG+MonsterfSTR)\times MonsterpDIF
\]

\[
D_{final}=\left\lfloor D\times
\max(0.5,1-PhysDT-DT)\right\rfloor
\]

---

## 2. Magic Damage

### Base magic damage

\[
dINT=INT_{caster}-INT_{target}
\]

\[
D=V+(dINT\times M)
\]

For negative dINT:

\[
D=V+dINT
\]

Above the applicable inflection point:

\[
D=V+\left(C+(dINT-C)\times\frac{M}{2}\right)
\]

### Magic damage calculation (stage pipeline)

Floor after every multiplication:

\[
\begin{aligned}
D_{final}=\lfloor\cdots\lfloor
&D\times MTDR\\
&\times Staff\\
&\times Affinity\\
&\times Resist\\
&\times RankReduction\\
&\times MB\\
&\times MBB\\
&\times DayWeather\\
&\times MABMDB\\
&\times TMDA\\
&\times Potency
\end{aligned}
\]

Stages are independent — not interchangeable bonuses. See §17.

### Magic attack / defense ratio

\[
cMAB=\frac{100+MAB}{100}
\]

\[
tMDB=\frac{100+MDB}{100}
\]

\[
MABMDB=\frac{cMAB}{tMDB}
\]

### Magic damage taken

\[
D_{final}=
\left\lfloor
\frac{
\left\lfloor D\times(1-MDT-DT)\right\rfloor
}{
1+MDB/100
}
\right\rfloor
\]

\[
D_{reduction}=\max(0.5,1-MDT-DT)
\]

---

## 3. Accuracy, Evasion & Hit Rate

### Accuracy (prefer §16)

\[
ACC=\lfloor DEX\times0.75\rfloor+Skill_{ACC}+Gear+Traits
\]

**Correction (§22):** replace DEX term with \(\lfloor DEX/2\rfloor\).

### Accuracy skill contribution (prefer §16 tiers)

\[
Skill_{ACC}=
\begin{cases}
Skill,&Skill\leq200\\
200+\lfloor(Skill-200)\times0.9\rfloor,&Skill>200
\end{cases}
\]

### Evasion

\[
EVA=\lfloor AGI/2\rfloor+Skill_{EVA}+Gear+Traits
\]

### Evasion skill contribution

\[
Skill_{EVA}=
\begin{cases}
Skill,&Skill\leq200\\
200+\lfloor(Skill-200)\times0.9\rfloor,&Skill>200
\end{cases}
\]

### Hit rate

\[
HitRate=75+\left\lfloor\frac{ACC-EVA}{2}\right\rfloor-2(dLVL)
\]

\[
HitRate_{final}=\min(95,\max(20,HitRate))
\]

### Critical hit rate (prefer §16)

\[
CritRate=5+\lfloor DEX/4\rfloor+CritBonus
\]

### Double attack rate

\[
DARate=1+\lfloor DEX/4\rfloor+DABonus
\]

---

## 4. Attack, Defense & Attributes

### Physical attack

\[
ATK=STR\times R_{STR}+WeaponATK+GearATK+TraitsATK
\]

### STR-to-attack conversion

\[
R_{STR}=
\begin{cases}
1.0,&2H/MainHand/Ranged\\
0.5,&OffHand\\
0.75,&HandToHand
\end{cases}
\]

### Magic attack

\[
MATK=INT+0.5(MND)+GearMATK+TraitsMATK
\]

### Physical defense

\[
DEF=VIT+GearDEF+TraitsDEF
\]

### Magic defense

\[
MDEF=0.5(INT)+MND+GearMDEF+TraitsMDEF
\]

### Attribute-adjusted attack ratio

\[
cRatio=\frac{ATK}{DEF}-0.05(dLVL)
\]

### Attribute-adjusted magic ratio

\[
cMDIF=\frac{MATK}{MDEF}
\]

---

## 5. Combat Skill Caps

### General skill cap

\[
SkillCap=BaseCap(Lv,Rank)
\]

| Rank | Levels 1–50 | Levels 51–60 | Levels 61–70 |
|------|-------------|--------------|--------------|
| A+ | \(3(L-1)+6\) | \(5(L-50)+153\) | \(4.85(L-60)+203\) |
| A− | \(3(L-1)+6\) | \(5(L-50)+153\) | \(4.10(L-60)+203\) |
| B+ | \(2.9(L-1)+5\) | \(4.9(L-50)+147\) | \(3.70(L-60)+196\) |
| B | \(2.9(L-1)+5\) | \(4.9(L-50)+147\) | \(3.23(L-60)+196\) |
| B− | \(2.9(L-1)+5\) | \(4.9(L-50)+147\) | \(2.70(L-60)+196\) |
| C+ | \(2.8(L-1)+5\) | \(4.8(L-50)+142\) | \(2.50(L-60)+190\) |
| C | \(2.8(L-1)+5\) | \(4.8(L-50)+142\) | \(2.25(L-60)+190\) |
| C− | \(2.8(L-1)+5\) | \(4.8(L-50)+142\) | \(2.00(L-60)+190\) |
| D | \(2.7(L-1)+4\) | \(4.7(L-50)+136\) | \(1.85(L-60)+183\) |
| E | \(2.5(L-1)+4\) | \(4.5(L-50)+126\) | \(1.95(L-60)+171\) |

Table covers levels 1–70 only. Extend before using at higher caps.

### Skill deficit

\[
SkillDeficit=SkillCap-CurrentSkill
\]

### Skill-up eligibility

\[
CurrentSkill<SkillCap
\]

---

## 6. Crafting

### Skill difference

\[
\Delta Skill=CraftingSkill-RecipeSkill
\]

### Skill-up calculation

\[
X=SkillCap-CurrentSkill-\sum Factors
\]

### Effective crafting skill

\[
EffectiveSkill=\lfloor CraftingSkill\rfloor
\]

### Success / HQ / skill-up

\[
SuccessRate=f(\Delta Skill)
\]

\[
HQRate=g(\Delta Skill)
\]

\[
P(SkillUp)=f(CurrentSkill,SkillCap,X)
\]

\[
SkillGain=
\begin{cases}
0.1,&CurrentSkill>60\\
Variable,&CurrentSkill\leq60
\end{cases}
\]

See §21 for explicit \(P_{success}\) and \(P_{HQ}\).

---

## 7. TP Generation

Prefer delay-based §14 over weapon-damage approximation.

### Approximate TP gain (legacy / provisional)

\[
TP=(WeaponDMG+fSTR)\times0.03
\]

### TP after additional attacks

\[
TP_{total}=\sum_{i=1}^{n}TP_i
\]

### TP accumulation

\[
TP_{final}=TP_{current}+TP_{gain}
\]

---

## 8. Enmity

### Cumulative enmity

\[
CE_{final}=\max(0,CE_{current}+CE_{action}-CE_{decay})
\]

### Volatile enmity

\[
VE_{final}=\max(0,VE_{current}+VE_{action}-VE_{decay})
\]

### Total enmity

\[
Enmity=CE+VE
\]

### Damage-based enmity (provisional)

\[
CE_{damage}=Damage\times CE_{coefficient}
\]

\[
VE_{damage}=Damage\times VE_{coefficient}
\]

Prefer action-specific coefficients (§20, §31).

---

## 9. Haste & Action Timing

### Effective haste (additive categories — §22)

\[
Haste_{total}=Haste_{equipment}+Haste_{magic}+Haste_{ability}
\]

\[
Haste_{effective}=\min(Haste_{total},0.80)
\]

### Reduced attack delay

\[
Delay_{final}=\lfloor Delay_{base}\times(1-Haste_{effective})\rfloor
\]

### Reduced cast / recast

\[
CastTime_{final}=CastTime_{base}\times(1-Haste_{cast})
\]

\[
Recast_{final}=Recast_{base}\times(1-Haste_{recast})
\]

§19 multiplicative equipment×magic form is an alternate stacking model; Black Anvil uses additive categories with an 80% effective cap unless a mode flag says otherwise.

---

## 10. Magic Burst & Elemental Modifiers

\[
MB_{total}=1+MB_{gear}+MB_{bonus}
\]

\[
DayMultiplier=1+DayBonus
\]

\[
WeatherMultiplier=1+WeatherBonus
\]

\[
DayWeather=DayMultiplier\times WeatherMultiplier
\]

\[
Affinity=1+AffinityBonus
\]

\[
Staff=1+StaffBonus
\]

\[
Resist=ResistanceMultiplier(Element,Target)
\]

\[
ElementalMultiplier=Staff\times Affinity\times DayWeather\times Resist
\]

---

## 11. Damage Reduction

\[
PhysReduction=\max(0.5,1-PhysDT-DT)
\]

\[
MagicReduction=\max(0.5,1-MDT-DT)
\]

\[
DamageReceived=\lfloor IncomingDamage\times PhysReduction\rfloor
\]

\[
DamageReceived=\left\lfloor IncomingDamage\times MagicReduction\right\rfloor
\]

\[
MDBMultiplier=\frac{1}{1+MDB/100}
\]

\[
DamageReceived=\lfloor Damage\times MagicReduction\times MDBMultiplier\rfloor
\]

---

## 12. Weapon Skill Calculations

### Level-adjusted WSC

\[
WSC=\left\lfloor
\left\lfloor\sum_i STAT_i\times Modifier_i\right\rfloor
\times\alpha(Lv)
\right\rfloor
\]

\[
\alpha(Lv)=1-0.01\left\lfloor\frac{Lv-1}{6}\right\rfloor
\]

(Level 1–75 conventional WSC table.)

### TP-dependent fTP interpolation

\[
fTP(TP)=fTP_1+
\frac{TP-TP_1}{TP_2-TP_1}
(fTP_2-fTP_1)
\]

### Multi-hit weapon skill damage

\[
D_{WS}=\sum_{i=1}^{n}
(WDMG+fSTR+WSC)\times fTP_i\times pDIF_i
\]

Subsequent hits generally use \(fTP=1.0\).

### Critical weapon skill damage

\[
D_{crit}=BaseDamage\times pDIF_{crit}
\]

### Ranged weapon skill damage

\[
D_{RangedWS}=(WDMG+AMG+fSTR2+WSC)
\times fTP\times pDIF_{ranged}
\]

---

## 13. Ranged Damage & Accuracy

\[
cRatio_{ranged}=
\frac{RangedATK}{TargetDEF}
-0.025(dLVL)
\]

\[
D_{ranged}=BaseDamage\times pDIF_{ranged}
\]

\[
RACC=\lfloor AGI/2\rfloor+
RangedSkill+Gear+Traits
\]

\[
HitRate_{ranged}=
75+\left\lfloor
\frac{RACC-TargetEVA}{2}
\right\rfloor-2(dLVL)
\]

\[
HitRate_{ranged}=
\min(95,\max(20,HitRate_{ranged}))
\]

---

## 14. Tactical Points — Delay Formula

Replace weapon-damage TP approximation with delay-based calculation.

### Base TP per hit

\[
TP_{base}(d)=
\begin{cases}
5+\frac{(d-180)\times1.5}{180},&d\leq180\\
5+\frac{(d-180)\times6.5}{270},&180<d\leq450\\
11.5+\frac{(d-450)\times1.5}{30},&450<d\leq480\\
13+\frac{(d-480)\times1.5}{50},&480<d\leq530\\
14.5+\frac{(d-530)\times3.5}{470},&d>530
\end{cases}
\]

### Store TP

\[
TP_{STP}=
\left\lfloor
10\times TP_{base}\times
(1+STP/100)
\right\rfloor/10
\]

### Total TP gained

\[
TP_{total}=\sum_{i=1}^{n}TP_{STP,i}
\]

### TP after a weapon skill

\[
TP_{remaining}=TP_{current}-TP_{WS}
\]

### TP returned by additional hits

\[
TP_{gain}=TP_{main}+TP_{offhand}+TP_{extra}
\]

Historical base TP formula; confirm applicable revision before treating as universal.

---

## 15. Physical Damage — pDIF Corrections

### Melee attack ratio

\[
Ratio=\min\left(\frac{ATK}{DEF},2.25\right)
\]

\[
cRatio=Ratio-0.05(dLVL)
\]

\[
cRatio_{ranged}=Ratio-0.025(dLVL)
\]

### Melee pDIF (simplified two-handed model)

\[
pDIF_{max}=
\begin{cases}
0.4+1.2r,&0\leq r<0.5\\
1,&0.5\leq r\leq5/6\\
1.25r,&5/6<r\leq10/6\\
1.2r,&10/6<r\leq2
\end{cases}
\]

\[
pDIF_{min}=
\begin{cases}
\max(0,-0.5+1.2r),&r<1.25\\
1,&1.25\leq r\leq1.5\\
-0.8+1.2r,&r>1.5
\end{cases}
\]

\[
pDIF_{final}=pDIF_{random}\times Random(1.00,1.05)
\]

\[
pDIF_{crit}=
\min(3,pDIF_{intermediate}+1)
\times Random(1.00,1.05)
\]

### Ranged pDIF

\[
pDIF_{max}=
\begin{cases}
\frac{10}{9}r,&r<0.9\\
1,&0.9\leq r\leq1.1\\
r,&1.1<r\leq3
\end{cases}
\]

\[
pDIF_{min}=
\begin{cases}
r,&r<0.9\\
1,&0.9\leq r\leq1.1\\
-\frac{3}{19}+\frac{20}{19}r,&1.1<r\leq3
\end{cases}
\]

Separate one-handed, two-handed, and ranged curves for production. Do not use §1 linear min/max as exact universal pDIF.

---

## 16. Accuracy — Additional Skill Scaling

\[
SkillACC=
\begin{cases}
Skill,&Skill\leq200\\
200+\lfloor0.9(Skill-200)\rfloor,&200<Skill\leq400\\
380+\lfloor0.8(Skill-400)\rfloor,&Skill>400
\end{cases}
\]

\[
ACC_{DEX}=\lfloor DEX/2\rfloor
\]

\[
ACC=\lfloor SkillACC+ACC_{DEX}+Gear+Traits+Effects\rfloor
\]

Apply the same tiered structure to evasion skill contribution.

### Critical hit rate from attributes

\[
CritRate=5+\left\lfloor
\frac{DEX-AGI_{target}}{4}
\right\rfloor+CritBonus
\]

\[
CritRate_{final}=\max(0,\min(100,CritRate))
\]

---

## 17. Magic Damage — Additional Components

\[
D=mDMG+V+(dINT\times M)
\]

\[
dINT=INT_{caster}-INT_{target}
\]

Offensive white magic:

\[
dMND=MND_{caster}-MND_{target}
\]

Independent stages:

- Multiple-target damage reduction (MTDR)
- Elemental affinity
- Species damage taken (SDT)
- Resist state
- Resistance rank
- Magic burst / MB gear bonus
- Day and weather
- MAB vs MDB
- Target magic damage adjustment (TMDA)
- Spell potency

\[
D_{MTDR}=\lfloor D\times MTDR\rfloor
\]

\[
D_{SDT}=\lfloor D\times SDT\rfloor
\]

\[
D_{resist}=\lfloor D\times Resist\rfloor
\]

\[
D_{rank}=\lfloor D\times RankReduction\rfloor
\]

\[
D_{TMDA}=\lfloor D\times TMDA\rfloor
\]

\[
D=V+(dINT\times M)+mDMG
\]

\[
D_{final}=\min(D,D_{cap})
\]

Floor after every multiplication.

---

## 18. Magic Weapon Skill Damage

\[
D_{base}=
\left(1.5(Lv+1)+2+WSC\right)\times fTP+fINT
\]

\[
fINT=8+\frac{\Delta INT}{2}
\]

\[
D_{WS}=D_{base}\times Resist\times Staff
\times Weather\times MDIF\times TMDA
\]

Item-level weapon adjustment:

\[
D_{iLvl}=2.45(iLvl-99)
\]

\[
D_{base}=
\left(
1.5(Lv+1)+2+D_{iLvl}+WSC
\right)\times fTP+fINT
\]

---

## 19. Haste — Stacking & Caps

\[
Haste_{category}=\sum Haste_i
\]

Alternate multiplicative form (equipment × magic):

\[
Haste_{total}=
1-(1-Haste_{equipment})(1-Haste_{magic})
\]

Black Anvil default: additive categories + 80% cap (§9 / §22).

\[
Haste_{effective}=\min(Haste_{total},Haste_{cap})
\]

\[
Delay_{final}=
\left\lfloor Delay_{base}
(1-Haste_{effective})\right\rfloor
\]

---

## 20. Enmity — Action-Based Calculation

\[
Enmity_{action}=CE_{action}+VE_{action}
\]

\[
CE_{modified}=CE_{base}\times(1+EnmityBonus)
\]

\[
VE_{modified}=VE_{base}\times(1+EnmityBonus)
\]

\[
VE_{final}=\max(0,VE_{current}-DecayRate\times\Delta t)
\]

\[
CE_{final}=\max(0,CE_{current}-CELoss)
\]

\[
TE=CE+VE
\]

\[
CELoss_{final}=CELoss_{base}\times(1-Reduction)
\]

---

## 21. Crafting — Explicit Probabilities

\[
P_{success}=
\begin{cases}
0.95,&\Delta Skill\geq0\\
0.95-0.05|\Delta Skill|,&-3\leq\Delta Skill<0\\
0.80-0.10(|\Delta Skill|-3),&-10\leq\Delta Skill<-3\\
0,&\Delta Skill<-10
\end{cases}
\]

\[
P_{HQ}=
\begin{cases}
0.015625,&0\leq\Delta Skill\leq10\\
0.0625,&11\leq\Delta Skill\leq30\\
0.25,&31\leq\Delta Skill\leq50\\
0.50,&\Delta Skill\geq51
\end{cases}
\]

\[
P_{NQ}=1-P_{HQ}
\]

\[
E[Output]=
P_{NQ}Q_{NQ}+
\sum_{i=1}^{n}P_{HQ_i}Q_{HQ_i}
\]

HQ probabilities apply only to successful synthesis outcomes.

---

## 22. Final Corrections to Existing Formulas

| Area | Replace | With |
|------|---------|------|
| ACC DEX | \(\lfloor 0.75\,DEX\rfloor\) | \(\lfloor DEX/2\rfloor\) |
| Skill ACC | 0.9 forever above 200 | tiers at 200 and 400 (§16) |
| Haste | multiplicative categories as default | additive + \(\min(\cdot,0.80)\) |
| WSC | raw stat % only | include \(\alpha(Lv)\) (§12) |
| TP | \(TP\approx(WDMG+fSTR)\times0.03\) | delay formula (§14) |
| pDIF | linear §1 | piecewise §15 by weapon class |
| Magic | single product | staged pipeline + mDMG/SDT/MTDR (§17) |
| Skill caps | 1–70 table only | extend through level cap |

---

## 23. Additional Formula Categories Still Needed

| Category | Missing calculations |
|----------|----------------------|
| Multi-hit attacks | Double, Triple, Quadruple Attack resolution |
| Dual wield | Main-hand/off-hand delay and damage |
| Hand-to-hand | Punch damage, kick attacks, extra hits |
| Critical damage | Critical modifiers and damage multipliers |
| Weapon skills | Multi-hit accuracy, TP scaling, fTP transfer |
| Ranged combat | Distance correction, ammunition, ranged attack delay |
| Magic accuracy | Magic skill, magic accuracy, resistance probability |
| Healing | Cure potency, MND/VIT scaling, healing caps |
| Enfeebling | Magic accuracy, resistance, duration |
| Enmity | Individual action-specific CE/VE coefficients |
| TP | Store TP, regain, TP bonus, TP overflow |
| Damage mitigation | Phalanx, Stoneskin, shields, block rates |
| Resistances | Elemental resistance ranks and partial resists |
| Pet combat | Pet attributes, skill, damage and TP |
| Status effects | Duration, potency, resistance and tick calculations |

---

## 24. Multi-Attack Calculations

\[
P_{MA}=P_{DA}+P_{TA}+P_{QA}
\]

(Simplified mutually exclusive model.)

\[
P(2\ hits)=P_{DA},\quad P(3\ hits)=P_{TA},\quad P(4\ hits)=P_{QA}
\]

\[
E[Hits]=1+P_{DA}+2P_{TA}+3P_{QA}
\]

\[
E[D]=P_1D_1+P_2D_2+P_3D_3+P_4D_4
\]

\[
E[TP]=\sum_{i=1}^{n}P_i\times TP_i
\]

\[
Rounds=\frac{TP_{Required}}{E[TP\ per\ Round]}
\]

Account for actual multi-attack priority before using as exact implementation.

---

## 25. Dual-Wield Calculations

\[
Delay_{combined}=Delay_{main}+Delay_{off}
\]

\[
Delay_{average}=\frac{Delay_{main}+Delay_{off}}{2}
\]

\[
Delay_{DW}=Delay_{average}(1-DW)
\]

\[
Delay_{final}=Delay_{DW}(1-Haste)
\]

\[
Time_{round}=\frac{Delay_{final}\times2}{60}
\]

\[
TP_{hand}=TPBase(Delay_{final})
\]

\[
TP_{round}=TP_{main}+TP_{off}
\]

\[
E[TP]=P_{mainHit}TP_{main}+P_{offHit}TP_{off}
\]

---

## 26. Hand-to-Hand Calculations

\[
Delay_{H2H}=Delay_{MartialArts}+Delay_{Weapon}
\]

\[
D_{punch}=(WeaponDMG+fSTR)\times pDIF
\]

\[
D_{round}=\sum_{i=1}^{n}D_{punch,i}
\]

\[
E[D_{round}]=E[Hits]\times E[D_{punch}]
\]

\[
TP_{round}=TPBase(Delay_{H2H})\times Hits
\]

\[
DPS=\frac{E[D_{round}]}{Time_{round}}
\]

---

## 27. Magic Accuracy & Resistance

\[
MagicACC=MagicSkill+StatACC+GearACC+TraitsACC
\]

\[
\Delta Stat=Stat_{caster}-Stat_{target}
\]

\[
StatACC=
\begin{cases}
\Delta Stat,&\Delta Stat\leq10\\
10+\frac{\Delta Stat-10}{2},&\Delta Stat>10
\end{cases}
\]

\[
\Delta MA=MagicACC-MagicEVA
\]

\[
MagicHitRate=
\begin{cases}
50+\Delta MA/2,&\Delta MA<0\\
50+\Delta MA,&\Delta MA\geq0
\end{cases}
\]

\[
MagicHitRate_{final}=\min(95,\max(5,MagicHitRate))
\]

### Partial resistance (unresisted probability \(p\))

\[
P(Full)=p
\]

\[
P(1/2)=p(1-p)
\]

\[
P(1/4)=p(1-p)^2
\]

\[
P(1/8)=(1-p)^3
\]

\[
\begin{aligned}
E[D]=D_{base}\bigg(&p+
\frac{p(1-p)}{2}\\
&+\frac{p(1-p)^2}{4}\\
&+\frac{(1-p)^3}{8}\bigg)
\end{aligned}
\]

---

## 28. Healing & Cure Calculations

\[
CurePower=
\left\lfloor\frac{MND}{2}\right\rfloor+
\left\lfloor\frac{VIT}{4}\right\rfloor+
HealingSkill
\]

\[
CurePower_{effective}=\min(CurePower,CurePowerCap)
\]

\[
CureBase=f(CurePower,SpellTier)
\]

\[
CureFinal=
\left\lfloor
CureBase\times
(1+CurePotency)
\times
(1+CureReceived)
\times DayWeather
\right\rfloor
\]

\[
HealingEfficiency=\frac{HP_{restored}}{MP_{consumed}}
\]

\[
Overheal=\max(0,CureFinal-TargetMissingHP)
\]

\[
EffectiveHealing=\min(CureFinal,TargetMissingHP)
\]

\[
HealingWaste=CureFinal-EffectiveHealing
\]

Base cure function is spell-tier-specific (floor, rate, soft cap, post-soft-cap scaling).

---

## 29. Stoneskin & Phalanx

\[
X=\left\lfloor\frac{EnhancingSkill}{3}+MND\right\rfloor
\]

\[
Stoneskin=
\begin{cases}
X,&X<80\\
2X-60,&80\leq X<130\\
3X-190,&X\geq130
\end{cases}
\]

\[
Stoneskin_{final}=\min(350,Stoneskin)
\]

\[
SS_{remaining}=\max(0,SS_{current}-DamageAbsorbed)
\]

\[
Phalanx=
\begin{cases}
\left\lfloor\frac{EnhancingSkill}{10}\right\rfloor-2,
&Skill\leq300\\
28+\left\lfloor\frac{Skill-300.5}{28.5}\right\rfloor,
&Skill>300
\end{cases}
\]

\[
Phalanx_{total}=Phalanx_{base}+Phalanx_{gear}
\]

\[
D_{final}=\max(0,D_{incoming}-Phalanx_{total})
\]

\[
D_{final}=
\max\left(
0,
\lfloor D_{incoming}(1-DT)\rfloor-Phalanx_{total}
\right)
\]

Phalanx reduces individual hits; Stoneskin is a finite absorption pool. Order of flat vs percentage reduction must match the chosen ruleset.

---

## 30. Shield Block & Damage Mitigation

\[
BlockRate=
BaseBlockRate+
(ShieldSkill-AttackerSkill)\times0.2325
\]

\[
P(Block)=\min(100,\max(0,BlockRate))
\]

\[
E[D]=P(Block)D_{blocked}+(1-P(Block))D_{unblocked}
\]

\[
D_{blocked}=\max(0,D_{incoming}-ShieldAbsorption)
\]

\[
ShieldAbsorption=ShieldBaseReduction+ShieldDefenseBonus
\]

\[
D_{final}=\left\lfloor D_{blocked}\times(1-PDT-DT)\right\rfloor
\]

\[
E[Reduction]=1-\frac{E[D]}{D_{incoming}}
\]

Block-rate coefficient is empirical; shield size affects probability and absorption.

---

## 31. Enmity — Cure / Damage Actions

\[
CE_{cure}=CE_{modifier}\times HP_{cured}
\]

\[
VE_{cure}=6\times CE_{modifier}\times HP_{cured}
\]

\[
TE_{cure}=CE_{cure}+VE_{cure}
\]

\[
CE_{final}=CE_{base}\times EnmityModifier
\]

\[
VE_{final}=VE_{base}\times EnmityModifier
\]

\[
CE_{damage}=f(Damage,ActionType,TargetLevel)
\]

\[
VE_{damage}=f(Damage,ActionType,TargetLevel)
\]

---

## 32. Enspell Damage

\[
E=
\begin{cases}
\left\lfloor\frac{6S}{100}\right\rfloor+3,&S\leq200\\
\left\lfloor\frac{5S}{100}\right\rfloor+5,&S>200
\end{cases}
\]

(\(S\) = Enhancing Magic skill.)

\[
E_{weather}=\left\lfloor E(1+Day+Weather)\right\rfloor
\]

\[
D_{Enspell}=E_{weather}+EquipmentBonus
\]

\[
E_{II}(n)=\min(2E,E+n)
\]

\[
D_{total}=\sum_{i=1}^{n}D_{Enspell,i}
\]

Model Enspell I/II independently from physical weapon damage.

---

## 33. Pet Combat Calculations

\[
PetATK=f(PetLevel,PetSTR,PetRank,Equipment,Traits,Buffs)
\]

\[
PetACC=PetSkill+PetDEX+PetGear+PetTraits
\]

\[
PetRatio=\frac{PetATK}{TargetDEF}
\]

\[
D_{pet}=(PetWeaponDMG+PetfSTR)\times PetpDIF
\]

\[
PetDPS=\frac{E[PetDamage]}{PetAttackCycle}
\]

\[
PetTP=\sum_{i=1}^{n}TP_{pet,i}
\]

Pet families may need separate scaling tables.

---

## 34. Enfeebling & Status Effects

\[
MA=EnfeeblingSkill+StatACC+GearMA+TraitsMA
\]

\[
P_{land}=f(MA-ME)
\]

\[
Duration_{final}=Duration_{base}\times DurationModifier
\]

\[
Duration_{resisted}=Duration_{final}\times ResistMultiplier
\]

\[
Duration_{remaining}=\max(0,Duration_{initial}-ElapsedTime)
\]

\[
DOT_{tick}=f(SpellPotency,TargetResistance,TargetModifiers)
\]

\[
DOT_{total}=DOT_{tick}\times TickCount
\]

\[
HP_{restored}=RegenPerTick\times TickCount
\]

\[
MP_{restored}=RefreshPerTick\times TickCount
\]

---

## 35. Ranged Attack Timing

\[
T_{aim}=\frac{WeaponDelay}{110}
\]

\[
T_{cycle}=T_{aim}+T_{recovery}+T_{free}
\]

\[
APS=\frac{1}{T_{cycle}}
\]

\[
DPS=APS\times E[D_{ranged}]
\]

\[
DPS_{effective}=APS\times HitRate\times E[D_{successful}]
\]

---

## 36. Remaining Calculation Dependencies

| System | Required calculation |
|--------|----------------------|
| Weapon skills | Individual weapon skill fTP tables |
| Multi-hit | Proc priority and independent hit resolution |
| Critical hits | Weapon-specific critical pDIF |
| Accuracy | Exact hit-rate correction and caps |
| Magic accuracy | Spell-specific landing and resistance |
| Healing | Individual Cure and Curaga tier functions |
| Enfeebling | Individual spell potency and duration |
| Enmity | Complete action-specific CE/VE tables |
| Shields | Exact shield-size and skill-tier formulas |
| Pets | Individual pet families and level scaling |
| Crafting | Exact skill-up probability and HQ outcome tables |
| Skill caps | Full rank tables through the applicable level cap |
| Ranged | Distance and attack-specific accuracy modifiers |
| Status effects | Individual tick intervals and duration rules |

---

## 37. Calculation Engine Architecture

```
CombatCalculation
├── CharacterStats
│   ├── Attributes
│   ├── CombatSkill
│   ├── Attack
│   ├── Defense
│   ├── Accuracy
│   └── Evasion
│
├── AttackResolution
│   ├── HitCheck
│   ├── CriticalCheck
│   ├── MultiAttackCheck
│   ├── BlockCheck
│   └── CounterCheck
│
├── PhysicalDamage
│   ├── BaseWeaponDamage
│   ├── fSTR
│   ├── WSC
│   ├── fTP
│   ├── cRatio
│   └── pDIF
│
├── MagicDamage
│   ├── BaseMagicDamage
│   ├── MagicAccuracy
│   ├── MagicResistance
│   ├── MABMDB
│   ├── ElementalModifiers
│   └── MagicDamageTaken
│
├── DamageMitigation
│   ├── PhysicalDT
│   ├── MagicDT
│   ├── ShieldReduction
│   ├── Stoneskin
│   └── Phalanx
│
├── ResourceGeneration
│   ├── TP
│   ├── MP
│   ├── HP
│   └── Enmity
│
└── PeriodicEffects
    ├── DamageOverTime
    ├── Regeneration
    ├── Refresh
    └── StatusDuration
```

---

## Runtime mapping

| Doc section | Current Rust surface (`src/rpg/`) | Status |
|-------------|-----------------------------------|--------|
| §1–4, §11 physical/magic stubs | `combat.rs`, `grades.rs` | Partial / provisional |
| §3 hit / crit | `hit_chance`, `crit_from_agi` | Provisional — align to §16 |
| Jobs / vitals | `jobs.rs`, `grades.rs`, `progression.rs` | Active |
| Edda boss realtime | `src/boss.rs` | Uses provisional melee until this doc is ported |

New combat code should cite the section number it implements.
