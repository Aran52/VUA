import { TERMS_PRESET_IDS,type TermsPresetId } from "@vua/contracts";
import { Icon } from "@vua/design-system";
import { strings,termsEverywhereCopy as copy } from "../../i18n/index.ts";
export function LibraryUseFilter({value,mode,disabled,onChange}:{value:TermsPresetId|"";mode:"available"|"all";disabled:boolean;onChange:(value:TermsPresetId|"",mode:"available"|"all")=>void}) {
  return <div className="vua-library-use-filter"><label className="vua-warehouse__field"><span>{copy.use}</span><div className="vua-warehouse__select"><select className="vua-warehouse__filter" value={value} disabled={disabled} onChange={e=>onChange(e.target.value as TermsPresetId|"",mode)}><option value="">{copy.allUses}</option>{TERMS_PRESET_IDS.map(id=><option key={id} value={id}>{strings.useTerms.presets[id]}</option>)}</select><Icon name="chevron-down" size={16} /></div></label>
    {value&&<label className="vua-warehouse__field"><span>{copy.mode}</span><div className="vua-warehouse__select"><select className="vua-warehouse__filter" value={mode} disabled={disabled} onChange={e=>onChange(value,e.target.value as "available"|"all")}><option value="available">{copy.available}</option><option value="all">{copy.annotated}</option></select><Icon name="chevron-down" size={16} /></div></label>}
  </div>;
}
