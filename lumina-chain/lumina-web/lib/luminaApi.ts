import { hexToBytes, numberArrayToU8a, u8aToNumberArray } from "./encoding";
import type { StablecoinInstruction, TxReceipt, Wallet } from "./types";
import { sign } from "./wallet";

type UnsignedTxRequest = {
  sender: number[];
  nonce: number;
  instruction: StablecoinInstruction;
  gas_limit: number;
  gas_price: number;
};

type Transaction = {
  sender: number[];
  nonce: number;
  instruction: StablecoinInstruction;
  signature: number[];
  gas_limit: number;
  gas_price: number;
};

async function j<T>(res: Response): Promise<T> {
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${res.status} ${res.statusText}: ${text}`);
  }
  return (await res.json()) as T;
}

export async function getState(apiBase: string) {
  return j<any>(await fetch(`${apiBase}/state`, { cache: "no-store" }));
}

export async function getHealth(apiBase: string) {
  return j<any>(await fetch(`${apiBase}/health`, { cache: "no-store" }));
}

export async function getAccount(apiBase: string, addressHex: string) {
  const addr = addressHex.trim().replace(/^0x/, "");
  const out = await j<any>(await fetch(`${apiBase}/account/${addr}`, { cache: "no-store" }));
  if (out && typeof out === "object" && "error" in out) {
    return {
      address: addressHex,
      lusd_balance: 0,
      ljun_balance: 0,
      lumina_balance: 0,
      nonce: 0,
      has_passkey: false,
      guardian_count: 0,
      has_pq: false,
      credit_score: 0,
      yield_positions: 0,
      active_streams: 0,
      custom_balances: {}
    };
  }
  return out;
}

export async function faucet(apiBase: string, addressHex?: string) {
  return j<any>(
    await fetch(`${apiBase}/faucet`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(addressHex ? { address: addressHex } : {})
    })
  );
}

async function getSigningBytesHex(apiBase: string, req: UnsignedTxRequest): Promise<string> {
  const res = await fetch(`${apiBase}/tx/signing_bytes`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(req)
  });
  const out = await j<{ signing_bytes_hex: string }>(res);
  return out.signing_bytes_hex;
}

export async function submitInstruction(
  apiBase: string,
  wallet: Wallet,
  instruction: StablecoinInstruction,
  opts?: { gasLimit?: number; gasPrice?: number }
): Promise<TxReceipt> {
  // Pull nonce from chain so the tx matches consensus check.
  const addressHex = Array.from(wallet.publicKey)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
  const acct = await getAccount(apiBase, addressHex);
  const nonce = typeof acct?.nonce === "number" ? acct.nonce : 0;

  const unsigned: UnsignedTxRequest = {
    sender: u8aToNumberArray(wallet.publicKey),
    nonce,
    instruction,
    gas_limit: opts?.gasLimit ?? 100_000,
    gas_price: opts?.gasPrice ?? 1
  };

  const signingHex = await getSigningBytesHex(apiBase, unsigned);
  const signingBytes = hexToBytes(signingHex);
  const sig = sign(wallet, signingBytes);

  const tx: Transaction = {
    ...unsigned,
    signature: u8aToNumberArray(sig)
  };

  const res = await fetch(`${apiBase}/tx`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(tx)
  });

  return j<TxReceipt>(res);
}
