import React from "react";
import { Meta, StoryObj } from "@storybook/react";
import { ScanForm } from "@/components/scanner/ScanForm";
import { within, userEvent, expect } from "@storybook/test";

/**
 * # ScanForm
 *
 * Primary interface for initiating security scans on Soroban smart contracts.
 * Supports multiple input methods: contract ID, WASM upload, and source code paste.
 *
 * ## Features
 * - Multi-tab input interface (Contract ID, Upload, Source Code)
 * - Real-time validation of Stellar addresses
 * - Drag-and-drop WASM file upload
 * - Source code editor with Rust syntax highlighting
 * - Scan configuration options (depth, rulesets)
 * - Recent scans quick-access
 *
 * ## Usage
 * ```tsx
 * <ScanForm
 *   onSubmit={handleScan}
 *   isScanning={isLoading}
 *   recentScans={recentScans}
 * />
 * ```
 *
 * ## Accessibility
 * - All form fields have associated labels
 * - Error messages are announced via aria-live
 * - File upload supports keyboard navigation
 * - Progress indicators during scan
 */
const meta: Meta<typeof ScanForm> = {
  title: "Components/Scanner/ScanForm",
  component: ScanForm,
  parameters: {
    layout: "centered",
    docs: {
      description: {
        component: "Form component for initiating smart contract security scans.",
      },
    },
  },
  tags: ["autodocs"],
  argTypes: {
    onSubmit: {
      action: "submitted",
      description: "Callback fired when scan is initiated",
    },
    isScanning: {
      control: "boolean",
      description: "Whether a scan is currently in progress",
      defaultValue: false,
    },
    recentScans: {
      control: "object",
      description: "List of recent scans for quick re-scan",
    },
    defaultTab: {
      control: "select",
      options: ["contract-id", "upload", "source"],
      description: "Default active tab",
      defaultValue: "contract-id",
    },
  },
};

export default meta;
type Story = StoryObj<typeof ScanForm>;

const mockRecentScans = [
  {
    id: "scan-001",
    contractId: "GAA...1234",
    timestamp: "2024-01-15T10:30:00Z",
    status: "completed" as const,
    vulnerabilitiesFound: 3,
  },
  {
    id: "scan-002",
    contractId: "GBB...5678",
    timestamp: "2024-01-14T15:45:00Z",
    status: "completed" as const,
    vulnerabilitiesFound: 0,
  },
];

/**
 * Default state - ready for input
 */
export const Default: Story = {
  args: {
    isScanning: false,
    recentScans: mockRecentScans,
    defaultTab: "contract-id",
  },
};

/**
 * Scanning state - shows progress
 */
export const Scanning: Story = {
  args: {
    isScanning: true,
    recentScans: mockRecentScans,
    defaultTab: "contract-id",
  },
};

/**
 * Upload tab - drag and drop interface
 */
export const UploadTab: Story = {
  args: {
    isScanning: false,
    recentScans: [],
    defaultTab: "upload",
  },
};

/**
 * Source code tab - code editor
 */
export const SourceCodeTab: Story = {
  args: {
    isScanning: false,
    recentScans: [],
    defaultTab: "source",
  },
};

/**
 * Empty state - no recent scans
 */
export const NoRecentScans: Story = {
  args: {
    isScanning: false,
    recentScans: [],
    defaultTab: "contract-id",
  },
};

/**
 * Interaction test - fill form and submit
 */
export const InteractionTest: Story = {
  args: {
    isScanning: false,
    recentScans: mockRecentScans,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    // Find and fill contract ID input
    const input = canvas.getByPlaceholderText("Enter contract ID...");
    await userEvent.type(input, "GAAQ3VJ2OPKZP7Q2X3Y4Z5A6B7C8D9E0F1");

    // Click scan button
    const scanButton = canvas.getByRole("button", { name: /start scan/i });
    await userEvent.click(scanButton);
  },
};