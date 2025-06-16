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
                    background-color: #2980b9 !important;
                }}
            </style>
        </head>
        <body style="margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background-color: #f5f5f5;">
            <table width="100%" cellpadding="0" cellspacing="0" style="background-color: #f5f5f5; padding: 40px 0;">
                <tr>
                    <td align="center">
                        <table width="600" cellpadding="0" cellspacing="0" style="background-color: white; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1);">
                            <!-- Header -->
                            <tr>
                                <td style="padding: 40px 40px 20px 40px; text-align: center;">
                                    <h1 style="margin: 0; color: #1a1a1a; font-size: 24px; font-weight: 600;">
                                        Uniframe Studio
                                    </h1>
                                </td>
                            </tr>
                            
                            <!-- Content -->
                            <tr>
                                <td style="padding: 0 40px 40px 40px;">
                                    <p style="margin: 0 0 20px 0; color: #4a4a4a; font-size: 16px; line-height: 24px;">
                                        Hi there!
                                    </p>
                                    <p style="margin: 0 0 30px 0; color: #4a4a4a; font-size: 16px; line-height: 24px;">
                                        Click the button below to securely sign in to your Uniframe Studio account:
                                    </p>
                                    
                                    <!-- Button -->
                                    <table width="100%" cellpadding="0" cellspacing="0">
                                        <tr>
                                            <td align="center" style="padding: 0 0 30px 0;">
                                                <a href="{}" 
                                                   class="button"
                                                   style="display: inline-block; 
                                                          padding: 14px 32px; 
                                                          background-color: #3498db; 
                                                          color: white; 
                                                          text-decoration: none; 
                                                          border-radius: 6px; 
                                                          font-weight: 600;
                                                          font-size: 16px;">
                                                    Sign In to Uniframe Studio
                                                </a>
                                            </td>
                                        </tr>
                                    </table>
                                    
                                    <p style="margin: 0 0 10px 0; color: #8a8a8a; font-size: 14px;">
                                        If the button doesn't work, copy and paste this link into your browser:
                                    </p>
                                    <p style="margin: 0 0 30px 0; word-break: break-all;">
                                        <a href="{}" style="color: #3498db; font-size: 14px;">{}</a>
                                    </p>
                                    
                                    <!-- Footer -->
                                    <table width="100%" cellpadding="0" cellspacing="0" style="border-top: 1px solid #e0e0e0; padding-top: 20px;">
                                        <tr>
                                            <td>
                                                <p style="margin: 0; color: #8a8a8a; font-size: 13px; line-height: 20px;">
                                                    🔒 This link expires in 1 hour for your security.
                                                </p>
                                                <p style="margin: 10px 0 0 0; color: #8a8a8a; font-size: 13px; line-height: 20px;">
                                                    If you didn't request this email, you can safely ignore it.
                                                </p>
                                            </td>
                                        </tr>
                                    </table>
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
