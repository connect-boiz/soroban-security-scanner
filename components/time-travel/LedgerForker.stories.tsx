import React from "react";
import { Meta, StoryObj } from "@storybook/react";
import { LedgerForker } from "@/components/time-travel/LedgerForker";

/**
 * # LedgerForker
 *
 * Interface for forking the Stellar network at a specific ledger sequence
 * to test contracts against historical state.
 *
 * ## Features
 * - Ledger sequence input with validation
 * - Network selection (Testnet, Mainnet, Futurenet)
 * - Fork progress indicator
 * - Historical state preview
 * - Save/load fork configurations
 *
 * ## Usage
 * ```tsx
 * <LedgerForker
 *   onFork={handleFork}
 *   networks={["testnet", "mainnet"]}
 *   maxLedger={5000000}
 * />
 * ```
 */
const meta: Meta<typeof LedgerForker> = {
  title: "Components/TimeTravel/LedgerForker",
  component: LedgerForker,
  parameters: {
    layout: "centered",
    docs: {
      description: {
        component: "Component for forking Stellar ledger at specific sequence.",
      },
    },
  },
  tags: ["autodocs"],
  argTypes: {
    onFork: {
      action: "forked",
      description: "Callback when fork is created",
    },
    networks: {
      control: "object",
      description: "Available networks",
    },
    maxLedger: {
      control: "number",
      description: "Maximum available ledger sequence",
    },
    isForking: {
      control: "boolean",
      description: "Whether fork operation is in progress",
      defaultValue: false,
    },
  },
};

export default meta;
type Story = StoryObj<typeof LedgerForker>;

export const Default: Story = {
  args: {
    networks: ["testnet", "mainnet", "futurenet"],
    maxLedger: 5000000,
    isForking: false,
  },
};

export const Forking: Story = {
  args: {
    networks: ["testnet"],
    maxLedger: 5000000,
    isForking: true,
  },
};

export const WithHistory: Story = {
  args: {
    networks: ["testnet", "mainnet"],
    maxLedger: 5000000,
    recentForks: [
      { ledger: 4500000, network: "testnet", timestamp: "2024-01-10" },
      { ledger: 3200000, network: "mainnet", timestamp: "2024-01-05" },
    ],
    isForking: false,
  },
};