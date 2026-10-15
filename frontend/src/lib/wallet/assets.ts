export type NetworkId = "all" | "ethereum" | "base" | "solana" | "bitcoin";

export type AssetId =
  | "ethereum"
  | "bitcoin"
  | "usd-coin"
  | "solana"
  | "chainlink"
  | "uniswap";

export type Asset = {
  id: AssetId;
  name: string;
  symbol: string;
  network: Exclude<NetworkId, "all">;
  networkLabel: string;
  decimals: number;
  coingeckoId: string;
};

export const ASSETS: Record<AssetId, Asset> = {
  ethereum: {
    id: "ethereum",
    name: "Ether",
    symbol: "ETH",
    network: "ethereum",
    networkLabel: "Ethereum",
    decimals: 6,
    coingeckoId: "ethereum",
  },
  bitcoin: {
    id: "bitcoin",
    name: "Bitcoin",
    symbol: "BTC",
    network: "bitcoin",
    networkLabel: "Bitcoin",
    decimals: 8,
    coingeckoId: "bitcoin",
  },
  "usd-coin": {
    id: "usd-coin",
    name: "USD Coin",
    symbol: "USDC",
    network: "ethereum",
    networkLabel: "Ethereum",
    decimals: 2,
    coingeckoId: "usd-coin",
  },
  solana: {
    id: "solana",
    name: "Solana",
    symbol: "SOL",
    network: "solana",
    networkLabel: "Solana",
    decimals: 4,
    coingeckoId: "solana",
  },
  chainlink: {
    id: "chainlink",
    name: "Chainlink",
    symbol: "LINK",
    network: "ethereum",
    networkLabel: "Ethereum",
    decimals: 4,
    coingeckoId: "chainlink",
  },
  uniswap: {
    id: "uniswap",
    name: "Uniswap",
    symbol: "UNI",
    network: "ethereum",
    networkLabel: "Ethereum",
    decimals: 4,
    coingeckoId: "uniswap",
  },
};

export const ASSET_LIST = Object.values(ASSETS);

export const NETWORKS: {
  id: NetworkId;
  label: string;
  short: string;
}[] = [
  { id: "all", label: "All networks", short: "All" },
  { id: "ethereum", label: "Ethereum", short: "ETH" },
  { id: "base", label: "Base", short: "Base" },
  { id: "solana", label: "Solana", short: "SOL" },
  { id: "bitcoin", label: "Bitcoin", short: "BTC" },
];

export const FALLBACK_PRICES: Record<
  AssetId,
  { usd: number; usd_24h_change: number }
> = {
  ethereum: { usd: 3524.18, usd_24h_change: 1.84 },
  bitcoin: { usd: 97240.0, usd_24h_change: 0.62 },
  "usd-coin": { usd: 1.0, usd_24h_change: 0.01 },
  solana: { usd: 178.42, usd_24h_change: -1.12 },
  chainlink: { usd: 18.36, usd_24h_change: 2.41 },
  uniswap: { usd: 8.94, usd_24h_change: -0.48 },
};

export const COLLECTIBLES = [
  {
    id: "study-01",
    name: "Study 01",
    collection: "Monolith",
    seed: 11,
  },
  {
    id: "orbit-7",
    name: "Orbit 7",
    collection: "Monolith",
    seed: 27,
  },
  {
    id: "field-a",
    name: "Field A",
    collection: "Index",
    seed: 44,
  },
  {
    id: "grid-12",
    name: "Grid 12",
    collection: "Index",
    seed: 62,
  },
] as const;
