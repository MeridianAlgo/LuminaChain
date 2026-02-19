"use client";

import Link from "next/link";
import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { getSession } from "../lib/auth";

export default function LandingPage() {
  const router = useRouter();

  useEffect(() => {
    if (getSession()) {
      router.replace("/dashboard");
    }
  }, [router]);

  return (
    <div className="landing-shell">
      <header className="landing-nav">
        <div className="landing-logo">Lumina</div>
        <div className="landing-nav-links">
          <Link href="/auth/login" className="ghost-link">Sign in</Link>
          <Link href="/auth/signup" className="hero-cta small">Get started</Link>
        </div>
      </header>

      <section className="landing-hero">
        <p className="landing-pill">Stablecoin-native infrastructure</p>
        <h1>Payments-grade stablecoin UX, with wallet-native identity.</h1>
        <p className="landing-sub">
          Sign in with email + password, then transact through a locally-linked self-custody wallet.
          Built for modern teams that want speed, transparency, and control.
        </p>
        <div className="landing-actions">
          <Link href="/auth/signup" className="hero-cta">Create account</Link>
          <Link href="/auth/login" className="hero-secondary">Sign in</Link>
        </div>
      </section>

      <section className="landing-grid">
        <article className="landing-card">
          <h3>1. Authenticate</h3>
          <p>Email/password gets you into your profile session quickly.</p>
        </article>
        <article className="landing-card">
          <h3>2. Wallet linked</h3>
          <p>On sign up, a wallet is generated and bound to your account in-browser.</p>
        </article>
        <article className="landing-card">
          <h3>3. Transact on-chain</h3>
          <p>Dashboard operations use the same typed Lumina instruction flow.</p>
        </article>
      </section>
    </div>
  );
}
