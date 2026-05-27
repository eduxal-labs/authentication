// ── Level constants ─────────────────────────────────────────────
export const Level = {
  Normal: 0,
  System: 1,
  Super: 2,
} as const;
export type Level = (typeof Level)[keyof typeof Level];

// ── Status constants ────────────────────────────────────────────
export const Status = {
  Invited: 0,
  Active: 1,
  Suspended: 2,
  Deleted: 3,
} as const;
export type Status = (typeof Status)[keyof typeof Status];

// ── User ────────────────────────────────────────────────────────
export interface User {
  id: string;
  phone: string;
  name: string;
  level: Level;
  status: Status;
  created: number;
  avatar_url: string | null;
  created_at: string | null;
  updated_at: string | null;
}

export interface VerificationCode {
  phone: string;
  code: string;
  purpose: "verification" | "change-phone";
  userId?: string;
  newPhone?: string;
}

export interface JwtPayload {
  sub: string | null;
  phone: string;
  purpose?: "registration" | "auth";
  level?: Level;
  iat: number;
  exp: number;
}

export interface SendCodeRequest {
  phone: string;
}

export interface VerifyCodeRequest {
  phone: string;
  code: string;
}

export interface RegisterRequest {
  phone: string;
  name: string;
}

export interface InviteRequest {
  phone: string;
  name: string;
}

export interface UpdateLevelRequest {
  level: Level;
}

export interface UpdateStatusRequest {
  status: Status;
}

export interface UpdateProfileRequest {
  name?: string;
}

export interface ChangePhoneRequest {
  new_phone: string;
}

export interface ChangePhoneVerifyRequest {
  new_phone: string;
  code: string;
}

export type AuthResponse =
  | {
      status: "existing_user";
      token: string;
      user: User;
    }
  | {
      status: "new_user";
      temp_token: string;
      phone: string;
    };

export interface ApiError {
  error: string;
  message: string;
}

export interface WhatsAppTemplateParams {
  code: string;
}
