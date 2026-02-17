# Dashboard UI Improvements Checklist

This checklist tracks the UI/UX improvements needed to make the NanoScale dashboard look like Vercel's design system.

## Layout & Structure

- [x] Add a sidebar navigation with items (Overview, Projects, Servers, Settings)
- [x] Add a proper header with logo, search bar, user avatar/dropdown, and breadcrumbs
- [x] Add a consistent page container with proper max-width constraints and responsive padding
- [x] Replace the empty home page with a dashboard showing recent activity, projects grid, and stats

## Typography & Colors

- [x] Add Geist font (Vercel's signature typeface)
- [x] Refine the color palette - use more subtle grays with better contrast ratios
- [x] Improve text hierarchy - better font sizes, weights, and line heights
- [x] Use proper accent colors - Vercel blue (#0070F3) for primary actions, subtle borders

## Components

- [x] Redesign buttons - add hover/focus states, subtle shadows, border radius tweaks
- [x] Improve input fields - better focus rings, placeholder styling, and transitions
- [x] Add proper card components - subtle shadows, hover effects, better border colors
- [x] Redesign the table - row hover states, better cell padding, sticky headers
- [x] Add dropdown menus (using Radix/Headless UI) instead of `<details>` hack
- [x] Add toast notifications instead of inline error messages
- [x] Add loading states/skeletons for async data

## Visual Polish

- [ ] Add subtle gradients - Vercel's signature gradient backgrounds and borders
- [x] Add favicon
- [x] Add smooth transitions - page transitions, hover effects
- [x] Improve modal styling - better backdrop blur, animations
- [x] Add empty states with illustrations when no data

## Pages to Improve

- [x] **Home page**: Add project cards grid, recent deployments, activity feed
- [x] **Servers page**: Add status badges, server cards view option, better metrics visualization
- [x] **New Project page**: Better form layout, validation feedback, Git provider selection UI
- [x] **Login/Setup pages**: Add branding, illustration, smoother transitions

## Missing Features (UX)

- [x] Add dark/light mode toggle
- [ ] Add keyboard shortcuts (cmd+k for search)
- [x] Add proper navigation highlighting for active routes
- [x] Add breadcrumb navigation
- [ ] Add responsive mobile menu
