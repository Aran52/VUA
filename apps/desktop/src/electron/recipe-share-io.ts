import fs from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { isShareImageOptions, normalizeShareCode, type ShareImageOptions, type ShareImageSaved, type ReceivedRecipeLink } from "@vua/contracts";
import { RecipeShareClient } from "./recipe-share-client.js";

export interface ShareArtifact { artifactId: string; byteLength: number; mediaType: "image/png" | "image/gif"; code?: string }
export interface ShareIOOptions {
  directory: string;
  client: RecipeShareClient;
  assertReady(): void;
  saveDialog(format: "png" | "gif"): Promise<string | null>;
  invoke(method: "amf.production.exportShareImage" | "amf.production.readShareImage", params: Record<string, unknown>): Promise<unknown>;
}
const MAX_IMAGE = 20 * 1024 * 1024;
export class RecipeShareIO {
  private readonly artifacts: string;
  constructor(private readonly options: ShareIOOptions) { this.artifacts = path.join(options.directory,"amf-production","share-images"); }
  private async checkedRoot(): Promise<string> {
    await fs.mkdir(this.artifacts,{recursive:true,mode:0o700});
    const root = await fs.realpath(this.artifacts), owner = await fs.realpath(this.options.directory);
    if (!root.startsWith(owner + path.sep)) throw new Error("share_image_invalid");
    return root;
  }
  private async artifact(value: unknown, maximum: number): Promise<{ file: string; bytes: Buffer; receipt: ShareArtifact }> {
    if(!value || typeof value!=="object")throw new Error("share_image_invalid");
    const receipt=value as ShareArtifact;
    if(typeof receipt.artifactId!=="string"||!/^share-[0-9a-f-]{36}\.(png|gif)$/u.test(receipt.artifactId)
      ||!Number.isSafeInteger(receipt.byteLength)||receipt.byteLength<1||receipt.byteLength>maximum)throw new Error("share_image_invalid");
    const file=path.join(this.artifacts,receipt.artifactId);
    const meta=await fs.lstat(file);
    const root=await this.checkedRoot(),actual=await fs.realpath(file);
    if(!meta.isFile()||meta.isSymbolicLink()||meta.size!==receipt.byteLength||!actual.startsWith(root+path.sep))throw new Error("share_image_invalid");
    const bytes=await fs.readFile(actual);
    const png=bytes.subarray(0,8).equals(Buffer.from([137,80,78,71,13,10,26,10]));
    const gif=["GIF87a","GIF89a"].includes(bytes.subarray(0,6).toString("ascii"));
    if(bytes.length!==receipt.byteLength||!(receipt.mediaType==="image/png"&&png&&file.endsWith(".png")||receipt.mediaType==="image/gif"&&gif&&file.endsWith(".gif")))throw new Error("share_image_invalid");
    return {file,bytes,receipt};
  }
  async exportImage(value: ShareImageOptions): Promise<ShareImageSaved> {
    this.options.assertReady();
    if(!isShareImageOptions(value))throw new Error("share_invalid");
    const destination=await this.options.saveDialog(value.format);
    if(destination===null)return {saved:false,url:null,code:null};
    this.options.assertReady();
    const published=value.document ? await this.options.client.publish(value.document,value.recipeId!) : null;
    let artifact: Awaited<ReturnType<RecipeShareIO["artifact"]>> | null=null;
    try {
      const result=await this.options.invoke("amf.production.exportShareImage",{schemaVersion:"0.1",runId:value.runId,format:value.format,index:value.frameIndex,code:published?.code??null});
      artifact=await this.artifact(result,8*1024*1024);
      if(artifact.receipt.mediaType!==`image/${value.format}`)throw new Error("share_image_invalid");
      this.options.assertReady();
      await fs.writeFile(destination,artifact.bytes);
      return {saved:true,url:published?.url??null,code:published?.code??null};
    } catch(error) {
      // The retained claim remains visible for explicit withdrawal after an image-save failure.
      throw new Error(published?"share_image_save_failed_published":error instanceof Error?error.message:"share_image_invalid");
    } finally { if(artifact)await fs.unlink(artifact.file).catch(()=>{}); }
  }
  async readImage(source: string): Promise<ReceivedRecipeLink> {
    this.options.assertReady();
    if(typeof source!=="string"||!path.isAbsolute(source))throw new Error("share_image_invalid");
    const metadata=await fs.lstat(source).catch(()=>{throw new Error("share_image_invalid");});
    if(!metadata.isFile()||metadata.isSymbolicLink()||metadata.size<1||metadata.size>MAX_IMAGE)throw new Error("share_image_too_large");
    const bytes=await fs.readFile(source);
    if(bytes.length>MAX_IMAGE)throw new Error("share_image_too_large");
    await this.checkedRoot();
    const inputId=`input-${randomUUID()}.bin`,input=path.join(this.artifacts,inputId);
    let artifact: Awaited<ReturnType<RecipeShareIO["artifact"]>> | null=null;
    try {
      await fs.writeFile(input,bytes,{flag:"wx",mode:0o600});
      this.options.assertReady();
      const result=await this.options.invoke("amf.production.readShareImage",{schemaVersion:"0.1",inputId});
      artifact=await this.artifact(result,1024*1024);
      const code=normalizeShareCode(artifact.receipt.code??"");
      if(!code||artifact.receipt.mediaType!=="image/png")throw new Error("share_image_no_code");
      const received=await this.options.client.read(`${this.options.client.origin}/r/${code}`);
      return {...received,preview:`data:image/png;base64,${artifact.bytes.toString("base64")}`};
    } finally { await fs.unlink(input).catch(()=>{});if(artifact)await fs.unlink(artifact.file).catch(()=>{}); }
  }
}
