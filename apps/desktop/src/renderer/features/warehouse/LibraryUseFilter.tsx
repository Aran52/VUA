import { TERMS_PRESET_IDS,type TermsPresetId } from "@vua/contracts";
import { strings,termsEverywhereCopy as copy } from "../../i18n/index.ts";
export function LibraryUseFilter({value,mode,disabled,onChange}:{value:TermsPresetId|"";mode:"available"|"all";disabled:boolean;onChange:(value:TermsPresetId|"",mode:"available"|"all")=>void}) {
  return <div className="vua-library-use-filter"><label className="vua-warehouse__field"><span>{copy.use}</span><select className="vua-warehouse__filter" value={value} disabled={disabled} onChange={e=>onChange(e.target.value as TermsPresetId|"",mode)}><option value="">{copy.allUses}</option>{TERMS_PRESET_IDS.map(id=><option key={id} value={id}>{strings.useTerms.presets[id]}</option>)}</select></label>
    {value&&<label className="vua-warehouse__field"><span>{copy.mode}</span><select className="vua-warehouse__filter" value={mode} disabled={disabled} onChange={e=>onChange(value,e.target.value as "available"|"all")}><option value="available">{copy.available}</option><option value="all">{copy.annotated}</option></select></label>}
  </div>;
}
