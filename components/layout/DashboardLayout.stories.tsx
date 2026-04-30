import React from "react";
import { Meta, StoryObj } from "@storybook/react";
import { DashboardLayout } from "@/components/layout/DashboardLayout";

/**
 * # DashboardLayout
 *
 * Main application layout providing navigation, header, and content area.
 * Responsive design with collapsible sidebar on mobile devices.
 *
 * ## Features
 * - Collapsible sidebar navigation
 * - Breadcrumb navigation
 * - User profile dropdown
 * - Notification center
 * - Theme toggle (light/dark)
 * - Responsive breakpoints
 *
 * ## Usage
 * ```tsx
 * <DashboardLayout>
 *   <YourPageContent />
 * </DashboardLayout>
 * ```
 */
const meta: Meta<typeof DashboardLayout> = {
  title: "Components/Layout/DashboardLayout",
  component: DashboardLayout,
  parameters: {
    layout: "fullscreen",
    docs: {
      description: {
        component: "Main dashboard layout with navigation and content areas.",
      },
    },
  },
  tags: ["autodocs"],
  argTypes: {
    children: {
      control: "text",
      description: "Page content",
    },
    user: {
      control: "object",
      description: "Current user data",
    },
    notifications: {
      control: "number",
      description: "Unread notification count",
    },
  },
};

export default meta;
type Story = StoryObj<typeof DashboardLayout>;

const mockUser = {
  name: "Alice Developer",
  email: "alice@stellar.dev",
  avatar: "https://github.com/alice.png",
  role: "security-researcher",
};

export const Default: Story = {
  args: {
    children: <div className="p-6">Page Content Here</div>,
    user: mockUser,
    notifications: 3,
  },
};

export const NoNotifications: Story = {
  args: {
    children: <div className="p-6">Page Content Here</div>,
    user: mockUser,
    notifications: 0,
  },
};

export const Mobile: Story = {
  args: {
    children: <div className="p-4">Mobile Page Content</div>,
    user: mockUser,
    notifications: 1,
  },
  parameters: {
    viewport: {
      defaultViewport: "mobile1",
    },
  },
};