pub fn create_magic_link_html(magic_link: &str) -> String {
    format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <style>
                .button:hover {{
                    background: linear-gradient(135deg, #2563eb 0%, #7c3aed 100%) !important;
                    transform: translateY(-1px);
                    box-shadow: 0 8px 25px rgba(59, 130, 246, 0.4) !important;
                }}
                .button {{
                    transition: all 0.2s ease !important;
                }}
            </style>
        </head>
        <body style="margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: linear-gradient(135deg, #0f0f23 0%, #1a1a2e 50%, #16213e 100%);">
            <table width="100%" cellpadding="0" cellspacing="0" style="background: linear-gradient(135deg, #0f0f23 0%, #1a1a2e 50%, #16213e 100%); padding: 15px 20px;">
                <tr>
                    <td align="center">
                        <table width="800" cellpadding="0" cellspacing="0" style="background: rgba(0, 0, 0, 0.4); backdrop-filter: blur(16px); border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 16px; box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3); max-width: 650px;">
                            
                            <!-- Header with gradient -->
                            <tr>
                                <td style="padding: 30px 40px 15px 40px; text-align: center; background: linear-gradient(135deg, rgba(59, 130, 246, 0.3) 0%, rgba(0, 0, 0, 0.8) 50%, rgba(139, 92, 246, 0.3) 100%); border-radius: 16px 16px 0 0; border-bottom: 1px solid rgba(255, 255, 255, 0.1);">
                                    <h1 style="margin: 0; color: white; font-size: 28px; font-weight: 700; text-shadow: 0 2px 4px rgba(0, 0, 0, 0.5);">
                                        ✨ Uniframe Studio
                                    </h1>
                                    <div style="width: 60px; height: 3px; background: linear-gradient(90deg, #3b82f6, #8b5cf6); margin: 16px auto 0; border-radius: 2px;"></div>
                                </td>
                            </tr>
                            
                            <!-- Content -->
                            <tr>
                                <td style="padding: 25px 40px 30px 40px; background: rgba(0, 0, 0, 0.6);">
                                    <p style="margin: 0 0 15px 0; color: #e5e5e5; font-size: 18px; line-height: 26px; font-weight: 500; text-align: center;">
                                        Welcome back! 👋
                                    </p>
                                    <p style="margin: 0 0 25px 0; color: rgba(255, 255, 255, 0.8); font-size: 16px; line-height: 24px; text-align: center;">
                                        Ready to create something amazing?<br>Click the button below to securely access your Uniframe Studio workspace:
                                    </p>
                                    
                                    <!-- Glowing Button -->
                                    <table width="100%" cellpadding="0" cellspacing="0">
                                        <tr>
                                            <td align="center" style="padding: 0 0 25px 0;">
                                                <a href="{}" 
                                                   class="button"
                                                   style="display: inline-block; 
                                                          padding: 16px 32px; 
                                                          background: linear-gradient(135deg, #3b82f6 0%, #8b5cf6 100%);
                                                          color: white; 
                                                          text-decoration: none; 
                                                          border-radius: 12px; 
                                                          font-weight: 600;
                                                          font-size: 16px;
                                                          box-shadow: 0 4px 15px rgba(59, 130, 246, 0.3);
                                                          border: 1px solid rgba(255, 255, 255, 0.2);">
                                                    Sign in
                                                </a>
                                            </td>
                                        </tr>
                                    </table>
                                    
                                    <!-- Fallback link with copy button -->
                                    <div style="background: rgba(255, 255, 255, 0.05); border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 8px; padding: 16px; margin: 16px 0; text-align: center;">
                                        <p style="margin: 0 0 8px 0; color: rgba(255, 255, 255, 0.7); font-size: 14px;">
                                            Button not working? Copy this magic link:
                                        </p>
                                        <div style="background: rgba(0, 0, 0, 0.3); padding: 10px; border-radius: 6px; border: 1px solid rgba(255, 255, 255, 0.1); text-align: center;">
                                            <a href="{}" style="color: #60a5fa; font-size: 14px; text-decoration: none; word-break: break-all;">{}</a>
                                        </div>
                                    </div>
                                    
                                    <!-- Security footer -->
                                    <table width="100%" cellpadding="0" cellspacing="0" style="border-top: 1px solid rgba(255, 255, 255, 0.1); padding-top: 16px; margin-top: 16px;">
                                        <tr>
                                            <td style="background: linear-gradient(135deg, rgba(16, 185, 129, 0.1) 0%, rgba(59, 130, 246, 0.1) 100%); border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 8px; padding: 14px;">
                                                <p style="margin: 0 0 6px 0; color: #10b981; font-size: 14px; line-height: 18px; font-weight: 600;">
                                                    🔒 Secure Authentication
                                                </p>
                                                <p style="margin: 0 0 6px 0; color: rgba(255, 255, 255, 0.7); font-size: 13px; line-height: 16px;">
                                                    This magic link expires in <strong style="color: #fbbf24;">1 hour</strong> for your security.
                                                </p>
                                                <p style="margin: 0; color: rgba(255, 255, 255, 0.6); font-size: 13px; line-height: 16px;">
                                                    Didn't request this? You can safely ignore this email.
                                                </p>
                                            </td>
                                        </tr>
                                    </table>
                                </td>
                            </tr>
                            
                            <!-- Footer -->
                            <tr>
                                <td style="padding: 16px 40px; text-align: center; background: linear-gradient(135deg, rgba(139, 92, 246, 0.2) 0%, rgba(0, 0, 0, 0.8) 50%, rgba(59, 130, 246, 0.2) 100%); border-radius: 0 0 16px 16px; border-top: 1px solid rgba(255, 255, 255, 0.1);">
                                    <p style="margin: 0; color: rgba(255, 255, 255, 0.5); font-size: 12px;">
                                        © 2025 Uniframe Studio - AI Video Processing Platform
                                    </p>
                                </td>
                            </tr>
                        </table>
                    </td>
                </tr>
            </table>
        </body>
        </html>
    "#,
        magic_link, magic_link, magic_link
    )
}