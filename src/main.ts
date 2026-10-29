// Booming Desktop — frontend bootstrap (M0 placeholder).
// Real UI ships in later milestones; this keeps the Vite build meaningful.
const app = document.querySelector<HTMLDivElement>("#app");

if (app) {
  app.innerHTML = `
    <h1>Booming Desktop</h1>
    <p>Frontend scaffold (M0) — engine wiring lands next.</p>
  `;
}
