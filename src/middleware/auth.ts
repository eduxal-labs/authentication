import { createMiddleware } from "hono/factory";
import { verifyToken } from "../services/jwt";
import type { JwtPayload } from "../types";
import { Level } from "../types";

type Env = {
  Bindings: {
    JWT_SECRET: string;
  };
  Variables: {
    jwtPayload: JwtPayload;
  };
};

export const requireAuth = createMiddleware<Env>(async (c, next) => {
  const header = c.req.header("Authorization");
  if (!header?.startsWith("Bearer ")) {
    return c.json(
      { error: "unauthorized", message: "Missing or invalid token" },
      401,
    );
  }

  const token = header.slice(7);
  try {
    const payload = await verifyToken(c.env, token);
    if (!payload.sub || payload.purpose !== "auth") {
      return c.json(
        { error: "unauthorized", message: "Invalid token type" },
        401,
      );
    }
    c.set("jwtPayload", payload);
    await next();
  } catch {
    return c.json(
      { error: "unauthorized", message: "Invalid or expired token" },
      401,
    );
  }
});

export const requireTempToken = createMiddleware<Env>(async (c, next) => {
  const header = c.req.header("Authorization");
  if (!header?.startsWith("Bearer ")) {
    return c.json(
      { error: "unauthorized", message: "Missing or invalid token" },
      401,
    );
  }

  const token = header.slice(7);
  try {
    const payload = await verifyToken(c.env, token);
    if (payload.sub !== null || payload.purpose !== "registration") {
      return c.json(
        { error: "unauthorized", message: "Invalid token type" },
        401,
      );
    }
    c.set("jwtPayload", payload);
    await next();
  } catch {
    return c.json(
      { error: "unauthorized", message: "Invalid or expired token" },
      401,
    );
  }
});

/** Requires auth + Super level. */
export const requireSuper = createMiddleware<Env>(async (c, next) => {
  const header = c.req.header("Authorization");
  if (!header?.startsWith("Bearer ")) {
    return c.json(
      { error: "unauthorized", message: "Missing or invalid token" },
      401,
    );
  }

  const token = header.slice(7);
  try {
    const payload = await verifyToken(c.env, token);
    if (!payload.sub || payload.purpose !== "auth") {
      return c.json(
        { error: "unauthorized", message: "Invalid token type" },
        401,
      );
    }
    if (payload.level !== Level.Super) {
      return c.json(
        { error: "forbidden", message: "Super access required" },
        403,
      );
    }
    c.set("jwtPayload", payload);
    await next();
  } catch {
    return c.json(
      { error: "unauthorized", message: "Invalid or expired token" },
      401,
    );
  }
});
