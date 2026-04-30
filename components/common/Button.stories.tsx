import React from "react";
import { Meta, StoryObj } from "@storybook/react";
import { Button } from "@/components/ui/Button";

/**
 * # Button
 *
 * Primary interactive element for user actions throughout the Soroban Security Scanner interface.
 * Supports multiple variants, sizes, and states to accommodate different interaction patterns.
 *
 * ## Usage Guidelines
 * - Use **primary** variant for main call-to-action actions (e.g., "Start Scan", "Submit")
 * - Use **secondary** variant for supporting actions
 * - Use **danger** variant for destructive actions (e.g., "Delete", "Remove")
 * - Use **ghost** variant for subtle actions within complex interfaces
 *
 * ## Accessibility
 * - All buttons have a minimum touch target of 44x44px
 * - Focus states are clearly visible with 2px outline
 * - Color contrast ratios meet WCAG 2.1 AA standards
 * - Disabled state uses `aria-disabled` for screen readers
 */
const meta: Meta<typeof Button> = {
  title: "Components/Common/Button",
  component: Button,
  parameters: {
    layout: "centered",
    docs: {
      description: {
        component: "Interactive button component with multiple variants and states.",
      },
    },
  },
  tags: ["autodocs"],
  argTypes: {
    variant: {
      control: "select",
      options: ["primary", "secondary", "danger", "ghost", "outline"],
      description: "Visual style variant of the button",
      table: {
        type: { summary: "string" },
        defaultValue: { summary: "primary" },
      },
    },
    size: {
      control: "select",
      options: ["sm", "md", "lg", "icon"],
      description: "Size of the button",
      table: {
        type: { summary: "string" },
        defaultValue: { summary: "md" },
      },
    },
    isLoading: {
      control: "boolean",
      description: "Shows loading spinner and disables interactions",
      table: {
        type: { summary: "boolean" },
        defaultValue: { summary: "false" },
      },
    },
    disabled: {
      control: "boolean",
      description: "Disables button interactions",
      table: {
        type: { summary: "boolean" },
        defaultValue: { summary: "false" },
      },
    },
    onClick: {
      action: "clicked",
      description: "Callback fired when button is clicked",
      table: {
        type: { summary: "() => void" },
      },
    },
    children: {
      control: "text",
      description: "Button label content",
    },
  },
};

export default meta;
type Story = StoryObj<typeof Button>;

/**
 * Default primary button - used for main actions
 */
export const Primary: Story = {
  args: {
    variant: "primary",
    size: "md",
    children: "Start Security Scan",
  },
};

/**
 * Secondary button - used for supporting actions
 */
export const Secondary: Story = {
  args: {
    variant: "secondary",
    size: "md",
    children: "View Documentation",
  },
};

/**
 * Danger button - used for destructive actions like deleting scans
 */
export const Danger: Story = {
  args: {
    variant: "danger",
    size: "md",
    children: "Delete Scan History",
  },
};

/**
 * Outline button - used for secondary actions in cards
 */
export const Outline: Story = {
  args: {
    variant: "outline",
    size: "md",
    children: "Configure Settings",
  },
};

/**
 * Ghost button - subtle action, often used in toolbars
 */
export const Ghost: Story = {
  args: {
    variant: "ghost",
    size: "md",
    children: "Cancel",
  },
};

/**
 * Loading state - shows spinner during async operations
 */
export const Loading: Story = {
  args: {
    variant: "primary",
    size: "md",
    isLoading: true,
    children: "Scanning Contract...",
  },
};

/**
 * Disabled state - prevents interaction
 */
export const Disabled: Story = {
  args: {
    variant: "primary",
    size: "md",
    disabled: true,
    children: "Scan in Progress",
  },
};

/**
 * Icon button - compact size for toolbars
 */
export const IconButton: Story = {
  args: {
    variant: "ghost",
    size: "icon",
    children: "🔍",
    "aria-label": "Search",
  },
};

/**
 * Size variations
 */
export const Sizes: Story = {
  render: () => (
    <div className="flex items-center gap-4">
      <Button size="sm">Small</Button>
      <Button size="md">Medium</Button>
      <Button size="lg">Large</Button>
    </div>
  ),
};