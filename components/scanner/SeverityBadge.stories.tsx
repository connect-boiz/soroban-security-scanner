import React from "react";
import { Meta, StoryObj } from "@storybook/react";
import { SeverityBadge } from "@/components/scanner/SeverityBadge";

/**
 * # SeverityBadge
 *
 * Visual indicator for vulnerability severity levels. Uses color coding and icons
 * to quickly communicate risk levels to users.
 *
 * ## Severity Levels
 * - **Critical** (Red): Immediate action required - contract is exploitable
 * - **High** (Orange): Should be fixed before deployment
 * - **Medium** (Yellow): Address in next release cycle
 * - **Low** (Blue): Informational, fix when convenient
 * - **Info** (Gray): Best practice suggestions
 *
 * ## Usage
 * ```tsx
 * <SeverityBadge severity="critical" showIcon />
 * <SeverityBadge severity="high" count={5} />
 * ```
 */
const meta: Meta<typeof SeverityBadge> = {
  title: "Components/Scanner/SeverityBadge",
  component: SeverityBadge,
  parameters: {
    layout: "centered",
    docs: {
      description: {
        component: "Badge component for displaying vulnerability severity levels.",
      },
    },
  },
  tags: ["autodocs"],
  argTypes: {
    severity: {
      control: "select",
      options: ["critical", "high", "medium", "low", "info"],
      description: "Severity level to display",
      table: {
        type: { summary: "SeverityLevel" },
        defaultValue: { summary: "info" },
      },
    },
    showIcon: {
      control: "boolean",
      description: "Whether to show the severity icon",
      defaultValue: true,
    },
    count: {
      control: "number",
      description: "Optional count to display (e.g., number of vulnerabilities)",
    },
    size: {
      control: "select",
      options: ["sm", "md", "lg"],
      description: "Badge size",
      defaultValue: "md",
    },
  },
};

export default meta;
type Story = StoryObj<typeof SeverityBadge>;

export const Critical: Story = {
  args: {
    severity: "critical",
    showIcon: true,
  },
};

export const High: Story = {
  args: {
    severity: "high",
    showIcon: true,
  },
};

export const Medium: Story = {
  args: {
    severity: "medium",
    showIcon: true,
  },
};

export const Low: Story = {
  args: {
    severity: "low",
    showIcon: true,
  },
};

export const Info: Story = {
  args: {
    severity: "info",
    showIcon: true,
  },
};

export const WithCount: Story = {
  args: {
    severity: "critical",
    count: 12,
    showIcon: true,
  },
};

export const WithoutIcon: Story = {
  args: {
    severity: "high",
    showIcon: false,
  },
};

export const AllSeverities: Story = {
  render: () => (
    <div className="flex flex-col gap-2">
      <SeverityBadge severity="critical" count={3} />
      <SeverityBadge severity="high" count={7} />
      <SeverityBadge severity="medium" count={12} />
      <SeverityBadge severity="low" count={5} />
      <SeverityBadge severity="info" count={8} />
    </div>
  ),
};