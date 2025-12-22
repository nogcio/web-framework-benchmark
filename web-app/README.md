# Web Framework Benchmark - Dashboard

This is the frontend UI for the Web Framework Benchmark project. It is a modern React application designed to visualize benchmark results and compare framework performance.

## Tech Stack

- **Framework**: React 19 + TypeScript
- **Build Tool**: Vite
- **Styling**: Tailwind CSS
- **State/Data Fetching**: TanStack Query (React Query)
- **Tables**: TanStack Table (React Table)
- **HTTP Client**: Axios
- **Icons**: Lucide React

## Prerequisites

- Node.js 18+
- The Rust API server running (for data access)

## Getting Started

### 1. Install Dependencies

```bash
cd web-app
npm install
```

### 2. Start the Backend

The dashboard relies on the Rust API server to fetch benchmark data. Make sure the backend is running on port 8080:

```bash
# In the project root
cargo run --release -- serve
```

### 3. Start the Development Server

```bash
npm run dev
```

The application will be available at http://localhost:5173.
The development server is configured to proxy requests starting with `/api` to `http://localhost:8080`.

## Scripts

- `npm run dev`: Start the development server.
- `npm run build`: Build the application for production.
- `npm run preview`: Preview the production build locally.
- `npm run lint`: Run ESLint.

## Project Structure

- `src/components`: Reusable UI components.
- `src/lib`: Utility functions and API clients.
- `src/store`: Global state management (if any).
- `src/types.ts`: TypeScript definitions for API responses and domain objects.
