export interface User {
  id: string;
  phone: string;
  name: string;
  avatar_url: string | null;
  created_at: string;
  updated_at: string;
}

export interface VerificationCode {
  phone: string;
  code: string;
  purpose: "verification" | "change-phone";
  userId?: string;
  newPhone?: string;
}

export interface JwtPayload {
  sub: string | null; // null for temp tokens
  phone: string;
  purpose?: "registration" | "auth";
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
