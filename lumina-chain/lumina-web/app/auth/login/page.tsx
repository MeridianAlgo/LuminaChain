"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { getSession, loginWithEmail, walletLogin } from "../../../lib/auth";
import { bytesToHex, hexToBytes } from "../../../lib/encoding";
import { newWallet, saveWallet } from "../../../lib/wallet";

export default function LoginPage() {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [importSkHex, setImportSkHex] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (getSession()) router.replace("/dashboard");
  }, [router]);

  async function handleEmailLogin(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const { wallet } = await loginWithEmail(email, password);
      saveWallet(wallet);
      router.replace("/dashboard");
    } catch (err: any) {
      setError(err?.message ?? "Unable to login");
    } finally {
      setBusy(false);
    }
  }

  async function importWalletOnly() {
    setBusy(true);
    setError(null);
    try {
      const raw = importSkHex.trim().toLowerCase().replace(/^0x/, "");
      const sk = hexToBytes(raw);
      const wallet = newWallet(sk);
      const publicKeyHex = bytesToHex(wallet.publicKey);
      saveWallet(wallet);
      walletLogin("0x" + publicKeyHex, publicKeyHex);
      router.replace("/dashboard");
    } catch (e: any) {
      setError(e?.message ?? "Invalid wallet secret key");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="auth-modern-shell">
      <div className="auth-modern-card">
        <p className="landing-pill">Welcome back</p>
        <h1>Sign in to Lumina</h1>
        <p className="auth-modern-sub">Use email/password for account access linked to your wallet.</p>

        <form onSubmit={handleEmailLogin} className="auth-modern-form">
          <label>Email</label>
          <input
            className="modern-input"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="you@company.com"
            required
          />

          <label>Password</label>
          <input
            className="modern-input"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="••••••••"
            required
          />

          <button className="hero-cta" disabled={busy} type="submit">
            {busy ? "Signing in..." : "Sign in"}
          </button>
        </form>

        <div className="auth-divider"><span>or wallet only</span></div>

        <div className="auth-modern-form">
          <label>Import secret key</label>
          <input
            className="modern-input"
            type="password"
            value={importSkHex}
            onChange={(e) => setImportSkHex(e.target.value)}
            placeholder="paste wallet secret key"
          />
          <button className="hero-secondary" disabled={busy || !importSkHex} onClick={importWalletOnly}>
            Enter with wallet
          </button>
        </div>

        {error && <div className="toast err">{error}</div>}

        <p className="auth-footnote">
          New user? <Link href="/auth/signup">Create account</Link>
        </p>
      </div>
    </div>
  );
}
