"use client";

import Link from "next/link";
import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { getSession } from "../lib/auth";

const marketTape = [
  { symbol: "USDT", price: "$0.9996", change: "-0.00%" },
  { symbol: "XRP", price: "$1.4000", change: "-4.58%" },
  { symbol: "BNB", price: "$602.8", change: "-2.18%" },
  { symbol: "USDC", price: "$0.9999", change: "-0.00%" },
  { symbol: "SOL", price: "$80.3", change: "-1.42%" }
];

const headlineStats = [
  { value: "$2.4T", title: "Global Market Cap", sub: "+3.2% past week" },
  { value: "$89B", title: "24h Trading Volume", sub: "Across all exchanges" },
  { value: "10,000+", title: "Active Assets", sub: "Tracked in real-time" },
  { value: "1.2M", title: "Data Points/Day", sub: "From 500+ exchanges" }
];

export default function LandingPage() {
  const router = useRouter();

  useEffect(() => {
    if (getSession()) {
      router.replace("/dashboard");
    }
  }, [router]);

  return (
    <div className="landing-shell nexus-theme">
      <header className="landing-nav nexus-nav">
        <div className="landing-logo-wrap">
          <div className="landing-logo-icon">↗</div>
          <div className="landing-logo">Lumina Nexus</div>
        </div>
        <div className="landing-nav-links">
          <Link href="/auth/login" className="ghost-link">Sign in</Link>
          <Link href="/auth/signup" className="hero-cta small">Get started</Link>
        </div>
      </header>

      <section className="landing-hero nexus-hero">
        <p className="landing-pill">● Live market data — updated every minute</p>
        <h1>Institutional-grade stablecoin analytics.</h1>
        <p className="landing-sub">
          Track your wallet portfolio, monitor reserves, and execute high-confidence stablecoin operations
          with the same precision infrastructure used by production trading teams.
        </p>
        <div className="landing-actions">
          <Link href="/auth/signup" className="hero-cta">Get started free →</Link>
          <Link href="/dashboard" className="hero-secondary">↗ View dashboard</Link>
        </div>
      </section>

      <section className="market-tape">
        {marketTape.map((m) => (
          <div key={m.symbol} className="ticker-item">
            <strong>{m.symbol}</strong>
            <span>{m.price}</span>
            <em>{m.change}</em>
          </div>
        ))}
      </section>

      <section className="headline-stats">
        {headlineStats.map((s) => (
          <article key={s.title} className="headline-card">
            <h3>{s.value}</h3>
            <p>{s.title}</p>
            <small>{s.sub}</small>
          </article>
        ))}
      </section>

      <section className="landing-grid">
        <article className="landing-card">
          <h3>Wallet-native identity</h3>
          <p>Email authentication plus local custody wallet flow for fast account onboarding.</p>
        </article>
        <article className="landing-card">
          <h3>Stablecoin execution</h3>
          <p>Mint, transfer, burn, and simulate flows via typed chain-native instructions.</p>
        </article>
        <article className="landing-card">
          <h3>Multi-asset visibility</h3>
          <p>Track LUSD, Lumina, and custom crypto balances from one unified dashboard.</p>
        </article>
      </section>
    </div>
  );
}
