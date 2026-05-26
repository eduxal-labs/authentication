import { Hono } from "hono";
import auth from "./routes/auth";
import user from "./routes/user";

type Bindings = {
  VERIFICATION_KV: KVNamespace;
  DB: D1Database;
  AVATARS_BUCKET: R2Bucket;
  PASETO_PASSWORD: string;
  WHATSAPP_PHONE_NUMBER_ID: string;
  WHATSAPP_TOKEN: string;
};

const app = new Hono<{ Bindings: Bindings }>();

// Health check
app.get("/", (c) => c.json({ status: "ok", service: "eduxal-auth-worker" }));

// Route mounting
app.route("/auth", auth);
app.route("/user", user);

export default app;
