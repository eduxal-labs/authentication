const WHATSAPP_API_VERSION = "v21.0";

export async function sendWhatsAppVerification(
  phoneNumberId: string,
  accessToken: string,
  recipient: string,
  code: string
): Promise<void> {
  const url = `https://graph.facebook.com/${WHATSAPP_API_VERSION}/${phoneNumberId}/messages`;

  const body = {
    messaging_product: "whatsapp",
    to: recipient,
    type: "template",
    template: {
      name: "auth_code",
      language: { code: "en" },
      components: [
        {
          type: "body",
          parameters: [{ type: "text", text: code }],
        },
        {
          type: "button",
          sub_type: "url",
          index: 0,
          parameters: [{ type: "text", text: code }],
        },
      ],
    },
  };

  const response = await fetch(url, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const errorBody = await response.text();
    throw new Error(
      `WhatsApp API error (${response.status}): ${errorBody}`
    );
  }

  // Log but don't await — non-critical
  response.json().catch(() => {});
}
