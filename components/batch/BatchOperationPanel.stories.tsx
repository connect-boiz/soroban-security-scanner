import React from "react";
import { Meta, StoryObj } from "@storybook/react";
import { BatchOperationPanel } from "@/components/batch/BatchOperationPanel";

/**
 * # BatchOperationPanel
 *
 * Interface for executing batch operations on multiple escrows or verifications.
 * Provides gas optimization estimates and progress tracking.
 *
 * ## Features
 * - Multi-select escrow/verification list
 * - Gas savings calculation (typically 40% vs individual operations)
 * - Operation type selection (release, verify, distribute)
 * - Progress tracking with real-time status updates
 * - Partial failure handling with detailed error reporting
 *
 * ## Usage
 * ```tsx
 * <BatchOperationPanel
 *   operationType="escrow-release"
 *   items={escrowItems}
 *   onExecute={handleBatchExecute}
 *   gasEstimate={gasEstimate}
 * />
 * ```
 */
const meta: Meta<typeof BatchOperationPanel> = {
  title: "Components/Batch/BatchOperationPanel",
  component: BatchOperationPanel,
  parameters: {
    layout: "padded",
    docs: {
      description: {
        component: "Panel for executing batch operations with gas optimization.",
      },
    },
  },
  tags: ["autodocs"],
  argTypes: {
    operationType: {
      control: "select",
      options: ["escrow-release", "verification", "distribution"],
      description: "Type of batch operation",
    },
    items: {
      control: "object",
      description: "List of items to process",
    },
    onExecute: {
      action: "executed",
      description: "Callback when batch is executed",
    },
    gasEstimate: {
      control: "object",
      description: "Gas estimate data",
    },
    isExecuting: {
      control: "boolean",
      description: "Whether batch execution is in progress",
      defaultValue: false,
    },
  },
};

export default meta;
type Story = StoryObj<typeof BatchOperationPanel>;

const mockEscrowItems = [
  { id: "ESC-001", amount: "1000", token: "XLM", receiver: "GAA...1234", status: "pending" },
  { id: "ESC-002", amount: "2500", token: "USDC", receiver: "GBB...5678", status: "pending" },
  { id: "ESC-003", amount: "500", token: "XLM", receiver: "GCC...9012", status: "pending" },
  { id: "ESC-004", amount: "10000", token: "BTC", receiver: "GDD...3456", status: "pending" },
];

const mockGasEstimate = {
  individualCost: "0.004 XLM",
  batchCost: "0.0024 XLM",
  savings: "40%",
  savingsAmount: "0.0016 XLM",
};

export const EscrowRelease: Story = {
  args: {
    operationType: "escrow-release",
    items: mockEscrowItems,
    gasEstimate: mockGasEstimate,
    isExecuting: false,
  },
};

export const Verification: Story = {
  args: {
    operationType: "verification",
    items: [
      { id: "VULN-001", contract: "GAA...1234", severity: "critical", bounty: "500" },
      { id: "VULN-002", contract: "GBB...5678", severity: "high", bounty: "250" },
    ],
    gasEstimate: {
      ...mockGasEstimate,
      individualCost: "0.003 XLM",
      batchCost: "0.0018 XLM",
    },
    isExecuting: false,
  },
};

export const Executing: Story = {
  args: {
    operationType: "escrow-release",
    items: mockEscrowItems.map((item, i) => ({
      ...item,
      status: i < 2 ? "completed" : "processing",
    })),
    gasEstimate: mockGasEstimate,
    isExecuting: true,
  },
};

export const EmptyState: Story = {
  args: {
    operationType: "escrow-release",
    items: [],
    gasEstimate: mockGasEstimate,
    isExecuting: false,
  },
};