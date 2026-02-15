# User Interface Specification: NanoScale

**Version:** 1.0.0  
**Framework:** Next.js 16 (App Router)  
**Component Library:** ShadCN UI (Radix Primitives + Tailwind CSS)  
**Theme:** Dark Mode Default (Zinc/Slate color palette)

## 1. Global Layout & Navigation

**Path:** `src/app/layout.tsx`  
**Description:** The persistent shell wrapping all authenticated pages.

### 1.1 Sidebar (Left Navigation)

- **Width:** Fixed 250px
- **Branding:** Top-left "NanoScale" logo (Text/Icon)
- **Navigation Links:**
	- Dashboard: Icon `LayoutDashboard`, Path `/`
	- Servers: Icon `Server`, Path `/servers`
	- Projects: Icon `Box`, Path `/projects`
	- Settings: Icon `Settings`, Path `/settings`
- **Footer:** User Avatar + Username (Dropdown: "Logout")

### 1.2 Header (Top Bar)

- **Breadcrumbs:** Dynamic path indication (e.g., `Servers > Hetzner-Worker-1`)
- **Action Area:** Right-aligned
- **Theme Toggle:** Sun/Moon icon
- **Docs Link:** External link to documentation

## 2. Authentication Pages

### 2.1 Initial Setup

**Path:** `/setup`  
**Condition:** Redirect here if API returns `users_count: 0`

**Components:**

- Card: Title "Welcome to NanoScale"
- Form:
	- `username` (Input)
	- `password` (Input, `type="password"`)
	- `confirm_password` (Input, `type="password"`)
- Submit Button: "Create Admin Account"

**Behavior:** On success, redirect to `/`.

### 2.2 Login

**Path:** `/login`

**Components:**

- Card: Title "Login"
- Form:
	- `username` (Input)
	- `password` (Input, `type="password"`)
- Submit Button: "Sign In"

**Behavior:** `POST /api/auth/login`. On success, redirect to `/`. On error, show Toast alert.

## 3. Core Pages

### 3.1 Dashboard Overview

**Path:** `/`  
**Purpose:** High-level cluster health at a glance.

**Components:**

- Metric Cards (Grid):
	- Total Servers: Count of active nodes.
	- Total Projects: Count of deployed apps.
	- Cluster CPU: Average load across nodes.
	- Cluster RAM: Total memory usage.
- Recent Activity Feed: List of last 5 deployment events (Project Name, Status, Time).
- System Status: Green/Red indicator for Orchestrator API health.

### 3.2 Server Management

#### 3.2.1 Server List

**Path:** `/servers`  
**Purpose:** View and add nodes.

**Components:**

- Header: Title "Servers", Button "Add Server" (opens modal)
- Data Table:
	- Columns: Name, IP Address, Status (Online/Offline), RAM Usage (ProgressBar), Actions (Three-dot menu: Edit, Delete)
- "Add Server" Modal:
	- Step 1: Button "Generate Join Token"
	- Step 2: Display shell command in a code block with copy button:

```bash
curl -sL nanoscale.sh | bash -s -- --join <token> --orchestrator <url>
```

	- Step 3: Polling indicator "Waiting for connection..."
	- Success: Closes modal, refreshes table.

#### 3.2.2 Server Details

**Path:** `/servers/[id]`  
**Purpose:** Deep dive into a specific node.

**Components:**

- Stats Grid: Real-time CPU/RAM/Disk graphs (Recharts)
- Hosted Projects: List of projects deployed specifically on this node
- Maintenance:
	- Button "Drain Node" (prevents new deploys)
	- Button "Remove Node"

### 3.3 Project Management

#### 3.3.1 Projects List

**Path:** `/projects`  
**Purpose:** Directory of all applications.

**Components:**

- Header: Title "Projects", Button "New Project"
- Grid/List Toggle: View projects as cards or table rows
- Project Card:
	- Thumbnail (Favicon/Screenshot)
	- Name + Production URL link
	- Status Badge (Live, Building, Error, Sleeping)
	- Repo Branch info

#### 3.3.2 New Project Wizard

**Path:** `/projects/new`  
**Purpose:** Onboarding a new app.

**Components:**

- Form Section 1: Source
	- `repo_url` (Input: HTTPS URL)
	- `branch` (Input: default `main`)
- Form Section 2: Configuration
	- `name` (Input: slug)
	- `build_command` (Input: default `bun run build`)
	- `server_id` (Select: dropdown of available servers)
- Form Section 3: Environment
	- Key/Value pair editor (Add Row/Remove Row)

**Submit Action:** `POST /api/projects`. Redirects to Project Details.

#### 3.3.3 Project Details

**Path:** `/projects/[id]`  
**Layout:** Tabbed Interface

**Tab 1: Overview**

- Header: Project Name, Live URL, "Deploy" Button, "Visit" Button
- Deployment Status: Latest commit hash, time, author
- Scale-to-Zero Config: Toggle switch "Enable Sleep Mode"
- Manual Trigger: Dropdown "Redeploy" -> "Clear Cache & Redeploy"

**Tab 2: Deployments**

- Table: History of builds
- Columns: Status (Success/Fail), Commit, Duration, Trigger (Git/Manual), Date
- Action: "View Logs" (opens Log Viewer Drawer)

**Tab 3: Real-time Logs**

- Component: Xterm.js canvas
- Behavior: Connects to SSE `/api/logs/[id]`. Displays stdout from the running service.
- Controls: "Clear", "Follow/Unfollow", "Download"

**Tab 4: Settings**

- General: Rename project
- Domains: Add custom domain input (triggers Nginx config update)
- Env Vars: Update `.env` file
- Danger Zone: "Delete Project" (Red button, requires typing project name to confirm)

## 4. Settings Page

**Path:** `/settings`  
**Purpose:** Orchestrator configuration.

**Components:**

- General: Site Name (for White-labeling)
- Profile: Change Password
- Updates: "Check for Updates" button (checks repo for new releases)