import { bytesToHex, hexToBytes } from "./encoding";
import type { Wallet } from "./types";

export type WalletAuth = {
  address: string;
  publicKey: string;
  email?: string;
  createdAt: number;
};

type Session = WalletAuth;

type LocalUser = {
  email: string;
  passwordHash: string;
  walletPublicKeyHex: string;
  walletSecretKeyHex: string;
  createdAt: number;
};

const SESSION_KEY = "lumina_wallet_session_v1";
const USERS_KEY = "lumina_email_users_v1";

function readJson<T>(key: string): T | null {
  if (typeof window === "undefined") return null;
  const raw = window.localStorage.getItem(key);
  if (!raw) return null;
  return JSON.parse(raw) as T;
}

function writeJson(key: string, value: unknown) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(key, JSON.stringify(value));
}

function normalizeEmail(email: string): string {
  return email.trim().toLowerCase();
}

async function sha256Hex(input: string): Promise<string> {
  const data = new TextEncoder().encode(input);
  const digest = await crypto.subtle.digest("SHA-256", data);
  const arr = Array.from(new Uint8Array(digest));
  return arr.map((x) => x.toString(16).padStart(2, "0")).join("");
}

function getUsers(): LocalUser[] {
  return readJson<LocalUser[]>(USERS_KEY) ?? [];
}

function setUsers(users: LocalUser[]) {
  writeJson(USERS_KEY, users);
}

export function getSession(): Session | null {
  return readJson<Session>(SESSION_KEY);
}

export function requireSession(): Session {
  const s = getSession();
  if (!s) throw new Error("Wallet not connected");
  return s;
}

export function logout() {
  if (typeof window === "undefined") return;
  window.localStorage.removeItem(SESSION_KEY);
  window.localStorage.removeItem("lumina_wallet_v1");
}

export function walletLogin(address: string, publicKey: string, email?: string) {
  const session: Session = {
    address,
    publicKey,
    email,
    createdAt: Date.now()
  };
  writeJson(SESSION_KEY, session);
}

export async function signupWithEmail(email: string, password: string, wallet: Wallet): Promise<Session> {
  const norm = normalizeEmail(email);
  if (!norm.includes("@")) throw new Error("Enter a valid email address");
  if (password.length < 8) throw new Error("Password must be at least 8 characters");

  const users = getUsers();
  if (users.some((u) => u.email === norm)) {
    throw new Error("This email is already registered");
  }

  const passwordHash = await sha256Hex(`${norm}:${password}`);
  const publicKeyHex = bytesToHex(wallet.publicKey);
  const secretKeyHex = bytesToHex(wallet.secretKey);

  users.push({
    email: norm,
    passwordHash,
    walletPublicKeyHex: publicKeyHex,
    walletSecretKeyHex: secretKeyHex,
    createdAt: Date.now()
  });
  setUsers(users);

  const session = {
    address: "0x" + publicKeyHex,
    publicKey: publicKeyHex,
    email: norm,
    createdAt: Date.now()
  };

  writeJson(SESSION_KEY, session);
  return session;
}

export async function loginWithEmail(email: string, password: string): Promise<{ session: Session; wallet: Wallet }> {
  const norm = normalizeEmail(email);
  const users = getUsers();
  const user = users.find((u) => u.email === norm);
  if (!user) throw new Error("No account found for that email");

  const passwordHash = await sha256Hex(`${norm}:${password}`);
  if (user.passwordHash !== passwordHash) throw new Error("Invalid email or password");

  const session = {
    address: "0x" + user.walletPublicKeyHex,
    publicKey: user.walletPublicKeyHex,
    email: user.email,
    createdAt: Date.now()
  };
  writeJson(SESSION_KEY, session);

  return {
    session,
    wallet: {
      publicKey: hexToBytes(user.walletPublicKeyHex),
      secretKey: hexToBytes(user.walletSecretKeyHex)
    }
  };
}
