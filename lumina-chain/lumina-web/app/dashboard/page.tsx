"use client";

import { useEffect, useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import { bytesToHex, hexToBytes, u8aToNumberArray } from "../../lib/encoding";
import { logout } from "../../lib/auth";
import { loadWallet, saveWallet } from "../../lib/wallet";
import { faucet, getAccount, getHealth, getState, submitInstruction } from "../../lib/luminaApi";
import type { AssetType, StablecoinInstruction, TxReceipt, Wallet } from "../../lib/types";

const DEFAULT_API = process.env.NEXT_PUBLIC_LUMINA_API ?? "http://localhost:3000";
const API_BASE_KEY = "lumina_api_base_v1";

// Tooltip component for hover explanations
function Tooltip({ children, text }: { children: React.ReactNode; text: string }) {
  const [show, setShow] = useState(false);
  return (
    <div className="tooltip-wrapper">
      <div 
        className="tooltip-trigger"
        onMouseEnter={() => setShow(true)}
        onMouseLeave={() => setShow(false)}
      >
        {children}
      </div>
      {show && <div className="tooltip-popup">{text}</div>}
    </div>
  );
}

// Info badge with hover tooltip
function InfoBadge({ text }: { text: string }) {
  return (
    <Tooltip text={text}>
      <span className="info-badge">ⓘ</span>
    </Tooltip>
  );
}

// Modern stat card with tooltip
function StatCard({ 
  label, 
  value, 
  tooltip,
  highlight = false 
}: { 
  label: string; 
  value: string | number; 
  tooltip: string;
  highlight?: boolean;
}) {
  return (
    <div className={`stat-card ${highlight ? 'highlight' : ''}`}>
      <div className="stat-header">
        <span className="stat-label">{label}</span>
        <InfoBadge text={tooltip} />
      </div>
      <div className="stat-value">{value ?? "—"}</div>
    </div>
  );
}

// Feature card with modern styling
function FeatureCard({ 
  title, 
  tooltip,
  children,
  badge
}: { 
  title: string;
  tooltip: string;
  children: React.ReactNode;
  badge?: string;
}) {
  return (
    <div className="feature-card">
      <div className="feature-card-header">
        <div className="feature-title-row">
          <h3 className="feature-title">{title}</h3>
          {badge && <span className="feature-badge">{badge}</span>}
        </div>
        <Tooltip text={tooltip}>
          <span className="info-icon">ⓘ</span>
        </Tooltip>
      </div>
      <div className="feature-content">
        {children}
      </div>
    </div>
  );
}

// Input with label and tooltip
function LabeledInput({
  label,
  tooltip,
  children
}: {
  label: string;
  tooltip: string;
  children: React.ReactNode;
}) {
  return (
    <div className="labeled-input">
      <div className="input-label-row">
        <label>{label}</label>
        <Tooltip text={tooltip}>
          <span className="input-info">ⓘ</span>
        </Tooltip>
      </div>
      {children}
    </div>
  );
}

export default function DashboardPage() {
  const router = useRouter();
  const [apiBase, setApiBase] = useState(DEFAULT_API);
  const [apiInitDone, setApiInitDone] = useState(false);
  const [wallet, setWallet] = useState<Wallet | null>(null);
  const [stateSummary, setStateSummary] = useState<any>(null);
  const [health, setHealth] = useState<any>(null);
  const [account, setAccount] = useState<any>(null);
  const [toast, setToast] = useState<{ kind: "ok" | "err"; msg: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [showDepositModal, setShowDepositModal] = useState(false);
  const [depositAmount, setDepositAmount] = useState(10000);
  const [apiStatus, setApiStatus] = useState<
    | { ok: true }
    | { ok: false; message: string }
    | null
  >(null);

  const addressHex = useMemo(() => {
    if (!wallet) return null;
    return "0x" + bytesToHex(wallet.publicKey);
  }, [wallet]);


  const customAssetRows = useMemo(() => {
    const balances = account?.custom_balances as Record<string, number> | undefined;
    if (!balances || typeof balances !== "object") return [];
    return Object.entries(balances)
      .filter(([, amount]) => typeof amount === "number" && amount > 0)
      .sort(([a], [b]) => a.localeCompare(b));
  }, [account]);

  useEffect(() => {
    const w = loadWallet();
    if (!w) {
      router.replace("/auth/login");
      return;
    }
    setWallet(w);
  }, [router]);

  useEffect(() => {
    if (typeof window === "undefined") return;
    const saved = window.localStorage.getItem(API_BASE_KEY);
    if (saved && saved.trim().length > 0) setApiBase(saved);
  }, []);

  function setApiBasePersist(next: string) {
    setApiBase(next);
    if (typeof window === "undefined") return;
    window.localStorage.setItem(API_BASE_KEY, next);
  }

  async function probeApiBase(base: string): Promise<{ ok: true } | { ok: false; message: string }> {
    try {
      const controller = new AbortController();
      const t = setTimeout(() => controller.abort(), 1200);
      const res = await fetch(`${base.replace(/\/$/, "")}/health`, { signal: controller.signal });
      clearTimeout(t);
      if (!res.ok) {
        return { ok: false, message: `HTTP ${res.status} from ${base}/health` };
      }
      return { ok: true };
    } catch (e: any) {
      const raw = e?.message ?? String(e);
      return { ok: false, message: raw };
    }
  }

  async function autoDiscoverApiBase() {
    const candidates = [
      apiBase,
      DEFAULT_API,
      "http://localhost:3000",
      "http://127.0.0.1:3000",
    ]
      .filter(Boolean)
      .map((x) => String(x).trim())
      .filter((x) => x.length > 0)
      .map((x) => x.replace(/\/$/, ""));

    const unique = Array.from(new Set(candidates));
    for (const base of unique) {
      const r = await probeApiBase(base);
      if (r.ok) {
        if (base !== apiBase) setApiBasePersist(base);
        setApiStatus({ ok: true });
        return;
      }
    }
  }

  function normalizeFetchError(e: any): string {
    const raw = e?.message ?? String(e);
    const lower = String(raw).toLowerCase();
    if (lower.includes("failed to fetch") || lower.includes("err_connection_refused")) {
      return `Cannot reach Lumina API at ${apiBase}. Make sure \"lumina-api\" is running on that URL/port (default http://localhost:3000).`;
    }
    if (lower.includes("404") && (lower.includes("/state") || lower.includes("/health"))) {
      return `Got 404 calling Lumina API endpoints at ${apiBase}. This usually means you're pointing at the Next.js web server (often http://localhost:3001) instead of lumina-api (default http://localhost:3000).`;
    }
    return raw;
  }

  async function refresh() {
    try {
      setToast(null);
      setApiStatus(null);
      const [s, h] = await Promise.all([getState(apiBase), getHealth(apiBase)]);
      setStateSummary(s);
      setHealth(h);
      if (addressHex) {
        const acct = await getAccount(apiBase, addressHex);
        setAccount(acct);
      }
      setApiStatus({ ok: true });
    } catch (e: any) {
      const msg = normalizeFetchError(e);
      setApiStatus({ ok: false, message: msg });
      setToast({ kind: "err", msg });
    }
  }

  useEffect(() => {
    if (!apiInitDone) return;
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [apiBase, addressHex, apiInitDone]);

  useEffect(() => {
    if (typeof window === "undefined") return;
    (async () => {
      await autoDiscoverApiBase();
      setApiInitDone(true);
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function doLogout() {
    logout();
    router.replace("/auth/login");
  }

  async function doClearWallet() {
    saveWallet(null);
    setWallet(null);
    setAccount(null);
    router.replace("/auth/login");
  }

  async function doDeposit() {
    if (!addressHex) {
      setToast({ kind: "err", msg: "No wallet loaded." });
      return;
    }
    if (apiStatus && apiStatus.ok === false) {
      setToast({ kind: "err", msg: apiStatus.message });
      return;
    }
    setBusy(true);
    setToast(null);
    try {
      const res = await faucet(apiBase, addressHex);
      setToast({ kind: "ok", msg: `Deposited ${res.amount} ${res.asset} to your wallet!` });
      setShowDepositModal(false);
      await refresh();
    } catch (e: any) {
      setToast({ kind: "err", msg: e?.message ?? String(e) });
    } finally {
      setBusy(false);
    }
  }

  async function send(si: StablecoinInstruction): Promise<TxReceipt | null> {
    if (!wallet) {
      setToast({ kind: "err", msg: "Create/import a wallet first." });
      return null;
    }

    if (apiStatus && apiStatus.ok === false) {
      setToast({ kind: "err", msg: apiStatus.message });
      return null;
    }

    setBusy(true);
    setToast(null);
    try {
      const receipt = await submitInstruction(apiBase, wallet, si);
      setToast({ kind: "ok", msg: `Submitted tx ${receipt.tx_id}` });
      await refresh();
      return receipt;
    } catch (e: any) {
      setToast({ kind: "err", msg: e?.message ?? String(e) });
      return null;
    } finally {
      setBusy(false);
    }
  }

  // Form states
  const [flashMintAmount, setFlashMintAmount] = useState(1000);
  const [flashMintCollateral, setFlashMintCollateral] = useState(1200);
  const [flashMintAsset, setFlashMintAsset] = useState<AssetType>({ LUSD: null });
  const [flashMintCommitmentHex, setFlashMintCommitmentHex] = useState("0".repeat(64));

  const [flashBurnAmount, setFlashBurnAmount] = useState(1000);

  const [instantRedeemAmount, setInstantRedeemAmount] = useState(100);
  const [instantRedeemDestHex, setInstantRedeemDestHex] = useState("".padStart(64, "0"));

  const [creditMintAmount, setCreditMintAmount] = useState(1000);
  const [creditMintCollateral, setCreditMintCollateral] = useState(1100);
  const [creditMintOracleHex, setCreditMintOracleHex] = useState("".padStart(64, "0"));
  const [creditMintMinScore, setCreditMintMinScore] = useState(750);
  const [creditMintProofHex, setCreditMintProofHex] = useState("");

  const [rwaDesc, setRwaDesc] = useState("Mock invoice #123");
  const [rwaValue, setRwaValue] = useState(50000);
  const [rwaElig, setRwaElig] = useState(true);
  const [rwaMaturity, setRwaMaturity] = useState<string>("");
  const [rwaProofHex, setRwaProofHex] = useState("");

  const [pledgeRwaId, setPledgeRwaId] = useState(0);
  const [pledgeAmount, setPledgeAmount] = useState(1000);

  return (
    <div className="dashboard-shell">
      {/* Top Navigation */}
      <nav className="dashboard-nav">
        <div className="nav-brand">
          <div className="nav-logo">
            <svg viewBox="0 0 40 40" fill="none" xmlns="http://www.w3.org/2000/svg">
              <circle cx="20" cy="20" r="18" stroke="url(#grad1)" strokeWidth="2"/>
              <path d="M12 20L18 26L28 14" stroke="url(#grad1)" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"/>
              <defs>
                <linearGradient id="grad1" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" stopColor="#7c5cff"/>
                  <stop offset="100%" stopColor="#35c27a"/>
                </linearGradient>
              </defs>
            </svg>
          </div>
          <span className="nav-title">Lumina</span>
        </div>
        <div className="nav-actions">
          <Tooltip text="Reload all data from the blockchain">
            <button className="nav-btn" onClick={refresh} disabled={busy}>
              <svg width="18" height="18" viewBox="0 0 20 20" fill="none">
                <path d="M4 10a6 6 0 0112 0 6 6 0 01-12 0zm12 0h-4m4 0v4M4 10h4M4 10V6" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"/>
              </svg>
              Refresh
            </button>
          </Tooltip>
          <Tooltip text="Disconnect wallet and return to login">
            <button className="nav-btn secondary" onClick={doLogout}>
              <svg width="18" height="18" viewBox="0 0 20 20" fill="none">
                <path d="M7 4l-4 4 4 4M3 8h10M13 4v12" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"/>
              </svg>
              Disconnect
            </button>
          </Tooltip>
        </div>
      </nav>

      <main className="dashboard-main">
        {/* Connection */}
        <section className="wallet-section">
          <div className="wallet-header">
            <h2 className="section-title">
              Connection
              <InfoBadge text="Point the UI at your running lumina-api. If you see connection refused, start the API or change the URL." />
            </h2>
          </div>

          <div className="wallet-grid" style={{ gridTemplateColumns: "2fr 1fr" }}>
            <div className="wallet-card">
              <div className="card-header">
                <span className="card-label">API Base URL</span>
                <InfoBadge text="Default is http://localhost:3000 (lumina-api). If your API runs elsewhere, change it here." />
              </div>
              <input
                className="modern-input"
                value={apiBase}
                onChange={(e) => setApiBasePersist(e.target.value)}
                placeholder="http://localhost:3000"
              />
              <div style={{ marginTop: 12, display: "flex", gap: 12, flexWrap: "wrap" }}>
                <Tooltip text="Re-check /state and /health using the API Base URL">
                  <button className="nav-btn" onClick={refresh} disabled={busy}>
                    Test connection
                  </button>
                </Tooltip>
                <Tooltip text="Clear wallet and go back to login">
                  <button className="nav-btn secondary" onClick={doClearWallet} disabled={busy}>
                    Clear wallet
                  </button>
                </Tooltip>
              </div>
            </div>

            <div className="stat-card">
              <div className="stat-header">
                <span className="stat-label">API Status</span>
                <InfoBadge text="Shows whether the UI can reach the API. If offline, start lumina-api and click Test connection." />
              </div>
              <div className="stat-value">
                {apiStatus?.ok === true ? "Online" : apiStatus?.ok === false ? "Offline" : "—"}
              </div>
              {apiStatus?.ok === false ? (
                <div className="p" style={{ marginTop: 8 }}>
                  {apiStatus.message}
                </div>
              ) : null}
            </div>
          </div>
        </section>

        {/* Wallet Overview Section */}
        <section className="wallet-section">
          <div className="wallet-header">
            <h2 className="section-title">
              Wallet Overview
              <InfoBadge text="Your wallet identity and balances. The faucet provides test funds for development." />
            </h2>
            <Tooltip text="Add test funds to your wallet for development">
              <button className="deposit-btn" onClick={() => setShowDepositModal(true)} disabled={busy}>
                <svg width="18" height="18" viewBox="0 0 20 20" fill="none">
                  <path d="M10 4v12M4 10h12" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
                </svg>
                Deposit Funds
              </button>
            </Tooltip>
          </div>

          <div className="wallet-grid">
            {/* Address Card */}
            <div className="wallet-card address-card">
              <div className="card-header">
                <span className="card-label">Your Address</span>
                <InfoBadge text="This is your Ed25519 public key (32 bytes). Share this to receive funds." />
              </div>
              <div className="address-display">
                <code>{addressHex ?? "Loading..."}</code>
                {addressHex && (
                  <Tooltip text="Copy address to clipboard">
                    <button 
                      className="copy-icon"
                      onClick={() => navigator.clipboard.writeText(addressHex)}
                    >
                      📋
                    </button>
                  </Tooltip>
                )}
              </div>
              <div className="address-actions">
                <Tooltip text="View your address on the explorer (when available)">
                  <button className="action-link">
                    View on Explorer →
                  </button>
                </Tooltip>
              </div>
            </div>

            {/* Balance Cards */}
            <StatCard 
              label="LUSD Balance" 
              value={account?.lusd_balance ?? 0}
              tooltip="Lumina USD - The stablecoin backed by collateral. Use for payments, trading, or as collateral."
              highlight
            />
            <StatCard 
              label="Lumina Balance" 
              value={account?.lumina_balance ?? 0}
              tooltip="Lumina token - The native chain token used for gas fees and governance."
            />
            <StatCard 
              label="Credit Score" 
              value={account?.credit_score ?? 0}
              tooltip="Your on-chain credit score. Higher scores unlock better rates for flash mints and lower collateral requirements."
            />
            <StatCard 
              label="Nonce" 
              value={account?.nonce ?? 0}
              tooltip="Transaction counter for your account. Increments with each transaction to prevent replay attacks."
            />
          </div>

          <div className="wallet-card" style={{ marginTop: 16 }}>
            <div className="card-header">
              <span className="card-label">Multi-Asset Balances</span>
              <InfoBadge text="Wallet support for non-native crypto assets registered as custom chain assets." />
            </div>
            {customAssetRows.length === 0 ? (
              <p className="muted">No custom assets yet. The simulation now supports BTC/ETH/SOL-style custom balances.</p>
            ) : (
              <div className="stack">
                {customAssetRows.map(([ticker, amount]) => (
                  <div key={ticker} className="row between">
                    <strong>{ticker}</strong>
                    <span>{amount}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        </section>

        {/* Chain Health Section */}
        <section className="chain-section">
          <h2 className="section-title">
            Chain Health
            <InfoBadge text="Real-time metrics about the Lumina blockchain health and stability" />
          </h2>
          <div className="chain-grid">
            <StatCard 
              label="Health Index" 
              value={health?.health_index ?? "—"}
              tooltip="Overall chain health score (0-100). Higher is better. Affected by reserves, liquidations, and circuit breakers."
              highlight
            />
            <StatCard 
              label="Health %" 
              value={health?.health_pct ?? "—"}
              tooltip="Health index displayed as percentage. Above 80% is healthy, below 50% triggers protective measures."
            />
            <StatCard 
              label="Reserve Ratio" 
              value={stateSummary?.reserve_ratio ?? "—"}
              tooltip="Ratio of collateral to outstanding stablecoins. Higher ratios mean more stability and safety."
            />
            <StatCard 
              label="Total LUSD Supply" 
              value={stateSummary?.total_lusd_supply ?? "—"}
              tooltip="Total amount of LUSD stablecoins in circulation across all accounts." 
            />
            <StatCard 
              label="Insurance Fund" 
              value={health?.insurance_fund_balance ?? "—"}
              tooltip="Funds set aside to cover bad debt and liquidations. Protects the protocol during market stress."
            />
            <StatCard 
              label="Circuit Breaker" 
              value={stateSummary?.circuit_breaker_active ? "Active" : "Inactive"}
              tooltip="Emergency protection that pauses certain functions if the protocol is at risk. Activates automatically."
              highlight={stateSummary?.circuit_breaker_active}
            />
            <StatCard 
              label="RWA Listings" 
              value={stateSummary?.rwa_listing_count ?? "—"}
              tooltip="Number of Real World Assets currently listed as available collateral in the marketplace."
            />
            <StatCard 
              label="Green Validators" 
              value={health?.green_validator_count ?? "—"}
              tooltip="Validators running on renewable energy. Supporting green validation earns extra rewards."
            />
          </div>
        </section>

        {/* Features Grid */}
        <section className="features-section">
          <h2 className="section-title">
            Features & Transactions
            <InfoBadge text="Submit transactions to interact with the Lumina protocol. Each transaction is signed in your browser." />
          </h2>
          
          <div className="features-grid">
            {/* Flash Mint */}
            <FeatureCard 
              title="Flash Mint" 
              tooltip="Borrow LUSD instantly without upfront collateral. Must repay (FlashBurn) in the same block or transaction fails."
              badge="⚡ Instant"
            >
              <LabeledInput label="Amount to Mint" tooltip="How much LUSD you want to flash mint. Must have sufficient collateral locked.">
                <input
                  className="modern-input"
                  type="number"
                  value={flashMintAmount}
                  onChange={(e) => setFlashMintAmount(Number(e.target.value))}
                />
              </LabeledInput>
              <LabeledInput label="Collateral Amount" tooltip="Amount of collateral to lock. Must be at least 110% of mint amount.">
                <input
                  className="modern-input"
                  type="number"
                  value={flashMintCollateral}
                  onChange={(e) => setFlashMintCollateral(Number(e.target.value))}
                />
              </LabeledInput>
              <LabeledInput label="Collateral Asset" tooltip="Which asset to use as collateral for this flash mint.">
                <select
                  className="modern-input"
                  value={Object.keys(flashMintAsset)[0]}
                  onChange={(e) => {
                    const k = e.target.value;
                    if (k === "LUSD") setFlashMintAsset({ LUSD: null });
                    else if (k === "LJUN") setFlashMintAsset({ LJUN: null });
                    else if (k === "Lumina") setFlashMintAsset({ Lumina: null });
                  }}
                >
                  <option value="LUSD">LUSD</option>
                  <option value="LJUN">LJUN</option>
                  <option value="Lumina">Lumina</option>
                </select>
              </LabeledInput>
              <Tooltip text="Submit the FlashMint transaction. Remember: you MUST FlashBurn in the same block!">
                <button
                  className="feature-btn primary"
                  disabled={busy}
                  onClick={() =>
                    send({
                      FlashMint: {
                        amount: flashMintAmount,
                        collateral_asset: flashMintAsset,
                        collateral_amount: flashMintCollateral,
                        commitment: u8aToNumberArray(hexToBytes(flashMintCommitmentHex).slice(0, 32))
                      }
                    })
                  }
                >
                  ⚡ Flash Mint
                </button>
              </Tooltip>
            </FeatureCard>

            {/* Flash Burn */}
            <FeatureCard 
              title="Flash Burn" 
              tooltip="Repay a flash mint to unlock your collateral. Must repay the FULL amount in the same block as the mint."
              badge="🔥 Repay"
            >
              <LabeledInput label="Amount to Burn" tooltip="Must match exactly the amount you flash minted. Partial repayments not allowed for flash mints.">
                <input
                  className="modern-input"
                  type="number"
                  value={flashBurnAmount}
                  onChange={(e) => setFlashBurnAmount(Number(e.target.value))}
                />
              </LabeledInput>
              <div className="burn-warning">
                <InfoBadge text="You must burn the FULL flash mint amount in the same block." />
                <span>Submit immediately after FlashMint</span>
              </div>
              <Tooltip text="Submit FlashBurn to repay and unlock collateral">
                <button
                  className="feature-btn danger"
                  disabled={busy}
                  onClick={() => send({ FlashBurn: { amount: flashBurnAmount } })}
                >
                  🔥 Flash Burn
                </button>
              </Tooltip>
            </FeatureCard>

            {/* Instant Redeem */}
            <FeatureCard 
              title="Instant Redeem" 
              tooltip="Redeem LUSD for underlying collateral instantly. Subject to redemption fees and available liquidity."
              badge="💰 Redeem"
            >
              <LabeledInput label="Amount to Redeem" tooltip="How much LUSD you want to redeem for underlying collateral.">
                <input
                  className="modern-input"
                  type="number"
                  value={instantRedeemAmount}
                  onChange={(e) => setInstantRedeemAmount(Number(e.target.value))}
                />
              </LabeledInput>
              <LabeledInput label="Destination Address" tooltip="The address (32-byte hex) to receive the redeemed collateral. Can be your address or another wallet.">
                <input
                  className="modern-input mono"
                  value={instantRedeemDestHex}
                  onChange={(e) => setInstantRedeemDestHex(e.target.value)}
                  placeholder="0x... (32 bytes)"
                />
              </LabeledInput>
              <Tooltip text="Submit redemption request. Collateral will be sent to destination address.">
                <button
                  className="feature-btn"
                  disabled={busy}
                  onClick={() =>
                    send({
                      InstantRedeem: {
                        amount: instantRedeemAmount,
                        destination: u8aToNumberArray(hexToBytes(instantRedeemDestHex).slice(0, 32))
                      }
                    })
                  }
                >
                  💰 Instant Redeem
                </button>
              </Tooltip>
            </FeatureCard>

            {/* Credit Score Mint */}
            <FeatureCard 
              title="Credit Score Mint" 
              tooltip="Mint with reduced collateral requirements based on your verified on-chain credit score."
              badge="📊 Credit"
            >
              <LabeledInput label="Amount to Mint" tooltip="How much LUSD to mint. Your credit score may reduce required collateral.">
                <input
                  className="modern-input"
                  type="number"
                  value={creditMintAmount}
                  onChange={(e) => setCreditMintAmount(Number(e.target.value))}
                />
              </LabeledInput>
              <LabeledInput label="Collateral" tooltip="Collateral amount. Good credit scores require less collateral (minimum may be 105% vs 110%).">
                <input
                  className="modern-input"
                  type="number"
                  value={creditMintCollateral}
                  onChange={(e) => setCreditMintCollateral(Number(e.target.value))}
                />
              </LabeledInput>
              <LabeledInput label="Min Credit Score" tooltip="Minimum credit score threshold for this transaction. Fails if your score is below this.">
                <input
                  className="modern-input"
                  type="number"
                  value={creditMintMinScore}
                  onChange={(e) => setCreditMintMinScore(Number(e.target.value))}
                />
              </LabeledInput>
              <Tooltip text="Submit Credit Score Mint transaction with your credit proof">
                <button
                  className="feature-btn"
                  disabled={busy}
                  onClick={() =>
                    send({
                      MintWithCreditScore: {
                        amount: creditMintAmount,
                        collateral_amount: creditMintCollateral,
                        credit_score_proof: u8aToNumberArray(hexToBytes(creditMintProofHex)),
                        min_score_threshold: creditMintMinScore,
                        oracle: u8aToNumberArray(hexToBytes(creditMintOracleHex).slice(0, 32))
                      }
                    })
                  }
                >
                  📊 Mint with Credit
                </button>
              </Tooltip>
            </FeatureCard>

            {/* List RWA */}
            <FeatureCard 
              title="List Real World Asset" 
              tooltip="Tokenize a real-world asset (invoice, property, etc.) to use as collateral. Requires attestation proof."
              badge="🏛️ RWA"
            >
              <LabeledInput label="Asset Description" tooltip="Human-readable description of the real-world asset being tokenized.">
                <input
                  className="modern-input"
                  value={rwaDesc}
                  onChange={(e) => setRwaDesc(e.target.value)}
                  placeholder="e.g., Invoice #12345"
                />
              </LabeledInput>
              <LabeledInput label="Attested Value" tooltip="The verified value of the asset in USD. Determines how much you can borrow against it.">
                <input
                  className="modern-input"
                  type="number"
                  value={rwaValue}
                  onChange={(e) => setRwaValue(Number(e.target.value))}
                />
              </LabeledInput>
              <LabeledInput label="Eligible as Collateral?" tooltip="Whether this asset can be used as collateral immediately after listing.">
                <select
                  className="modern-input"
                  value={rwaElig ? "true" : "false"}
                  onChange={(e) => setRwaElig(e.target.value === "true")}
                >
                  <option value="true">Yes</option>
                  <option value="false">No</option>
                </select>
              </LabeledInput>
              <Tooltip text="Submit the RWA listing to the marketplace">
                <button
                  className="feature-btn"
                  disabled={busy}
                  onClick={() =>
                    send({
                      ListRWA: {
                        asset_description: rwaDesc,
                        attested_value: rwaValue,
                        attestation_proof: u8aToNumberArray(hexToBytes(rwaProofHex)),
                        maturity_date: rwaMaturity.trim() === "" ? null : Number(rwaMaturity),
                        collateral_eligibility: rwaElig
                      }
                    })
                  }
                >
                  🏛️ List RWA
                </button>
              </Tooltip>
            </FeatureCard>

            {/* Use RWA as Collateral */}
            <FeatureCard 
              title="Use RWA as Collateral" 
              tooltip="Lock an RWA you own as collateral to borrow against it. The RWA must be listed and eligible."
              badge="🔒 Collateral"
            >
              <LabeledInput label="RWA ID" tooltip="The ID number of the RWA asset you want to use as collateral.">
                <input
                  className="modern-input"
                  type="number"
                  value={pledgeRwaId}
                  onChange={(e) => setPledgeRwaId(Number(e.target.value))}
                  placeholder="RWA listing ID"
                />
              </LabeledInput>
              <LabeledInput label="Amount to Pledge" tooltip="How much of the RWA value you want to use as collateral. Cannot exceed attested value.">
                <input
                  className="modern-input"
                  type="number"
                  value={pledgeAmount}
                  onChange={(e) => setPledgeAmount(Number(e.target.value))}
                />
              </LabeledInput>
              <Tooltip text="Lock this RWA as active collateral for borrowing">
                <button
                  className="feature-btn"
                  disabled={busy}
                  onClick={() =>
                    send({
                      UseRWAAsCollateral: {
                        rwa_id: pledgeRwaId,
                        amount_to_pledge: pledgeAmount
                      }
                    })
                  }
                >
                  🔒 Pledge RWA
                </button>
              </Tooltip>
            </FeatureCard>
          </div>
        </section>

        {/* Toast Notifications */}
        {toast && (
          <div className={`toast-float ${toast.kind}`}>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              {toast.kind === 'ok' ? (
                <path d="M8 0a8 8 0 100 16A8 8 0 008 0zm3.5 6.5L7 11l-2.5-2.5 1-1L7 9l3.5-3.5 1 1z"/>
              ) : (
                <path d="M8 0a8 8 0 100 16A8 8 0 008 0zm0 12a1 1 0 110-2 1 1 0 010 2zm0-3a1 1 0 01-.995-.89L7 8V4a1 1 0 012 0v4l-.005.11A1 1 0 018 9z"/>
              )}
            </svg>
            {toast.msg}
          </div>
        )}
      </main>

      {/* Deposit Modal */}
      {showDepositModal && (
        <div className="modal-overlay" onClick={() => setShowDepositModal(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3>💰 Deposit Funds</h3>
              <Tooltip text="Close this modal">
                <button className="modal-close" onClick={() => setShowDepositModal(false)}>×</button>
              </Tooltip>
            </div>
            <div className="modal-body">
              <p className="modal-desc">
                This is a development faucet. It adds free test funds to your wallet so you can test the protocol.
              </p>
              <div className="deposit-preview">
                <div className="deposit-row">
                  <span>To Address</span>
                  <code className="deposit-address">{addressHex?.slice(0, 12)}...{addressHex?.slice(-8)}</code>
                </div>
                <div className="deposit-row">
                  <span>Amount</span>
                  <strong className="deposit-amount">10,000 LUSD</strong>
                </div>
                <div className="deposit-row">
                  <span>Type</span>
                  <span>Faucet (Dev Only)</span>
                </div>
              </div>
              <div className="modal-actions">
                <Tooltip text="Add 10,000 LUSD to your wallet for testing">
                  <button className="login-btn primary" onClick={doDeposit} disabled={busy}>
                    {busy ? 'Processing...' : '✓ Confirm Deposit'}
                  </button>
                </Tooltip>
                <button className="login-btn secondary" onClick={() => setShowDepositModal(false)}>
                  Cancel
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
