"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { getSession, signupWithEmail } from "../../../lib/auth";
import { newWallet, saveWallet } from "../../../lib/wallet";

export default function SignupPage() {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (getSession()) router.replace("/dashboard");
  }, [router]);

  async function handleSignup(e: React.FormEvent) {
    e.preventDefault();
    setError(null);

    if (password !== confirmPassword) {
      setError("Passwords do not match");
      return;
    }

    setBusy(true);
    try {
      const wallet = newWallet();
      await signupWithEmail(email, password, wallet);
      saveWallet(wallet);
      router.replace("/dashboard");
    } catch (err: any) {
      setError(err?.message ?? "Unable to create account");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="auth-modern-shell">
      <div className="auth-modern-card">
        <p className="landing-pill">Create account</p>
        <h1>Launch your Lumina wallet identity</h1>
        <p className="auth-modern-sub">
          Your account uses email/password for access, then links to a generated local wallet.
        </p>

        <form onSubmit={handleSignup} className="auth-modern-form">
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
            placeholder="minimum 8 characters"
            required
            minLength={8}
          />

          <label>Confirm password</label>
          <input
            className="modern-input"
            type="password"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            placeholder="repeat password"
            required
            minLength={8}
          />

          <button className="hero-cta" disabled={busy} type="submit">
            {busy ? "Creating account..." : "Create account + wallet"}
          </button>
        </form>

        {error && <div className="toast err">{error}</div>}

        <p className="auth-footnote">
          Already registered? <Link href="/auth/login">Sign in</Link>
        </p>
      </div>
    </div>
  );
}
