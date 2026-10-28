import { readdirSync,readFileSync } from "node:fs";
import { expect,it } from "vitest";
import { isLibraryListParamsV02,isLibraryListV02 } from "./library-view-v02.js";
import { isDesktopGatewayRequestV1 } from "./desktop-gateway.js";
it("consumes both faces of the versioned library-use vectors",()=>{
  const root=new URL("../../../schemas/library-view/v0.2/examples/",import.meta.url);
  for(const name of readdirSync(root)){const value=JSON.parse(readFileSync(new URL(name,root),"utf8"));const valid=name.includes(".valid.");if(name.endsWith("request.json")){expect(isLibraryListParamsV02(value.params),name).toBe(valid);expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"use",...value}),name).toBe(valid);}else expect(isLibraryListV02(value),name).toBe(valid);}
});
it("rejects malformed annotations while retaining absent new fields",()=>{
  const value={schemaVersion:"0.2",total:0,offset:0,limit:50,items:[]};expect(isLibraryListV02(value)).toBe(true);
  expect(isLibraryListParamsV02({schemaVersion:"0.2",use:"vrchat_personal",useMode:"all",future:true})).toBe(false);
  expect(isLibraryListParamsV02({schemaVersion:"0.1",use:"vrchat_personal"})).toBe(false);
});
