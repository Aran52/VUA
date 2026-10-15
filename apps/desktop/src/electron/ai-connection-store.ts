import fs from "node:fs";
import path from "node:path";

interface SecretStorage {
  isEncryptionAvailable(): boolean;
  encryptString(value: string): Buffer;
  decryptString(value: Buffer): string;
}
/** No plaintext fallback. Without OS encryption pairing lasts only for this launch. */
export function aiConnectionStore(directory: string, secrets: SecretStorage) {
  const target = path.join(directory, "ai-connection.json");
  return {
    load(): { enabled: boolean; token: string | null } {
      try {
        if (!secrets.isEncryptionAvailable()) return { enabled: false, token: null };
        const value: unknown = JSON.parse(fs.readFileSync(target, "utf8"));
        if (!value || typeof value !== "object" || !("encryptedToken" in value) || !("enabled" in value)
          || typeof value.encryptedToken !== "string" || value.encryptedToken.length > 4096 || typeof value.enabled !== "boolean") throw new Error("invalid_store");
        const token = secrets.decryptString(Buffer.from(value.encryptedToken, "base64"));
        if (!/^[A-Za-z0-9_-]{43}$/.test(token)) throw new Error("invalid_pairing");
        return { enabled: value.enabled, token };
      } catch { return { enabled: false, token: null }; }
    },
    save(enabled: boolean, token: string): void {
      if (!secrets.isEncryptionAvailable()) return;
      fs.mkdirSync(directory, { recursive: true });
      const temp = `${target}.tmp`;
      fs.writeFileSync(temp, JSON.stringify({ version: "0.1", enabled, encryptedToken: secrets.encryptString(token).toString("base64") }), { mode: 0o600 });
      fs.renameSync(temp, target);
    },
  };
}
