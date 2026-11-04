import { afterEach, expect, it, vi } from "vitest";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { RecipeShareIO } from "./recipe-share-io.ts";
import { RecipeShareClient } from "./recipe-share-client.ts";
const directories: string[]=[];
afterEach(async()=>{for(const dir of directories.splice(0))await fs.rm(dir,{recursive:true,force:true});});
async function fixture(cancel=false) {
  const directory=await fs.mkdtemp(path.join(os.tmpdir(),"vua-share-io-"));directories.push(directory);
  const destination=path.join(directory,"chosen.png");
  const invoke=vi.fn(async()=>{const artifactId="share-00000000-0000-0000-0000-000000000000.png";const root=path.join(directory,"amf-production","share-images");await fs.mkdir(root,{recursive:true});const bytes=Buffer.from([137,80,78,71,13,10,26,10]);await fs.writeFile(path.join(root,artifactId),bytes);return {artifactId,byteLength:bytes.length,mediaType:"image/png"};});
  const client=new RecipeShareClient({directory,origin:"https://ph-r.com",fetch:vi.fn(),assertReady:()=>{}});
  return {directory,destination,invoke,io:new RecipeShareIO({directory,client,assertReady:()=>{},invoke,saveDialog:async()=>cancel?null:destination})};
}
it("cancels before publishing or rendering and cleans the opaque derived artifact after save",async()=>{
  const cancelled=await fixture(true);expect((await cancelled.io.exportImage({runId:"run",format:"png",frameIndex:0})).saved).toBe(false);expect(cancelled.invoke).not.toHaveBeenCalled();
  const f=await fixture();expect((await f.io.exportImage({runId:"run",format:"png",frameIndex:59})).saved).toBe(true);
  expect(await fs.readFile(f.destination)).toHaveLength(8);expect(await fs.readdir(path.join(f.directory,"amf-production","share-images"))).toEqual([]);
});
it("rejects oversize native files before invoking the image decoder",async()=>{
  const f=await fixture();const source=path.join(f.directory,"too-large.jpg");const file=await fs.open(source,"w");await file.truncate(20*1024*1024+1);await file.close();
  await expect(f.io.readImage(source)).rejects.toThrow("share_image_too_large");expect(f.invoke).not.toHaveBeenCalled();
  await expect(f.io.readImage("../outside.png")).rejects.toThrow("share_image_invalid");
});
