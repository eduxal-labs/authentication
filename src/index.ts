import { Hono } from "hono";
import auth from "./routes/auth";
import user from "./routes/user";
import admin from "./routes/admin";

type Bindings = {
  VERIFICATION_KV: KVNamespace;
  DB: D1Database;
  AVATARS_BUCKET: R2Bucket;
  JWT_SECRET: string;
  WHATSAPP_PHONE_NUMBER_ID: string;
  WHATSAPP_TOKEN: string;
};

const app = new Hono<{ Bindings: Bindings }>();

// Health check
app.get("/", (c) => c.json({ status: "ok", service: "eduxal-auth-worker" }));

// Route mounting
app.route("/auth", auth);
app.route("/user", user);
app.route("/admin", admin);

// Global error handler — catches unhandled exceptions and returns JSON
app.onError((err, c) => {
  console.error("Unhandled error:", err);
  return c.json(
    { error: "server_error", message: err.message, stack: err.stack },
    500,
  );
});

export default app;
