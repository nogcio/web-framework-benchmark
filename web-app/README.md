# web-app — Frontend for Web Framework Benchmark

This is the frontend UI for the web-framework-benchmark repository. It is a small React + TypeScript application scaffolded with Vite and styled with Tailwind CSS. The app is intended for exploring and visualizing benchmark data and related frontend views used alongside the main benchmarking tooling in this repository.

Key technologies:

- React 19 + TypeScript
- Vite for development and production builds
- Tailwind CSS for styling
- TanStack React Query and React Table for data fetching and tables
- Axios for HTTP requests

Quick scripts

- `npm run dev` — start the Vite development server (HMR enabled)
- `npm run build` — run TypeScript build and produce a production bundle via Vite
- `npm run preview` — locally preview the production build
- `npm run lint` — run ESLint across the project

Running locally

1. Install dependencies:

```bash
cd web-app
npm install
```

2. Start development server:

```bash
npm run dev
```

The dev server runs on `http://localhost:5173` by default. Open the URL in your browser to view the app.