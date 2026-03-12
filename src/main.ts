// Entrypoint (minimal). Run with: `deno run --allow-net src/main.ts` or `deno task start`
console.log("App started: src/main.ts");

export {};
import { Hono } from "hono";

const app = new Hono();

app.get("/", (c) => {
  return c.text("Hello Hono!");
});

Deno.serve(app.fetch);
