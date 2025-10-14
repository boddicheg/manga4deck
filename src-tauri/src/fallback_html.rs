pub fn get_fallback_html() -> &'static str {
    r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>Manga4Deck</title>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <style>
            body { font-family: Arial, sans-serif; margin: 40px; background: #1a1a1a; color: white; }
            h1 { color: #4CAF50; }
            .container { max-width: 800px; margin: 0 auto; }
            .status { background: #333; padding: 20px; border-radius: 8px; margin: 20px 0; }
            .api-test { background: #444; padding: 15px; border-radius: 5px; margin: 10px 0; }
            .error { background: #ff4444; padding: 15px; border-radius: 5px; margin: 10px 0; }
            .form-group { margin-bottom: 20px; }
            .form-group label { display: block; margin-bottom: 8px; font-size: 18px; font-weight: bold; }
            .form-group input { width: 100%; padding: 12px; background: #2a2a2a; border: 1px solid #555; border-radius: 4px; color: white; font-size: 16px; }
            .form-group input:focus { outline: none; border-color: #4CAF50; }
            .form-group p { color: #aaa; font-size: 14px; margin-top: 5px; }
            .button { background: #4CAF50; color: white; padding: 12px 24px; border: none; border-radius: 4px; cursor: pointer; font-size: 16px; margin-right: 10px; }
            .button:hover { background: #45a049; }
            .button.secondary { background: #666; }
            .button.secondary:hover { background: #555; }
            .button:disabled { opacity: 0.5; cursor: not-allowed; }
            .message { padding: 10px; border-radius: 4px; margin: 10px 0; }
            .message.success { background: #4CAF50; color: white; }
            .message.error { background: #f44336; color: white; }
            .message.info { background: #2196F3; color: white; }
            .flex { display: flex; justify-content: space-between; align-items: center; }
        </style>
    </head>
    <body>
        <div class="container">
            <h1>Manga4Deck Settings</h1>

            <form id="kavita-form" class="form">
                <h2>Configure Kavita API</h2>
                
                <div class="form-group">
                    <label for="serverIP">Kavita Server URL:</label>
                    <input type="text" id="serverIP" placeholder="e.g. 192.168.1.100:5001" />
                    <p>The URL of your Kavita server, including port number</p>
                </div>
                
                <div class="form-group">
                    <label for="username">Kavita Username:</label>
                    <input type="text" id="username" placeholder="Your Kavita username" />
                </div>
                
                <div class="form-group">
                    <label for="password">Kavita Password:</label>
                    <input type="password" id="password" placeholder="Enter your Kavita password" />
                    <p>Leave empty to keep current password if already stored</p>
                </div>
                
                <div class="form-group">
                    <label for="apiKey">Kavita API Key:</label>
                    <input type="text" id="apiKey" placeholder="Enter your Kavita API key" />
                    <p>API key can be found in your Kavita user settings</p>
                </div>
                
                <div class="flex">
                    <button type="button" onclick="loadSettings()" class="button secondary">Load Current Settings</button>
                    <button type="submit" id="saveBtn" class="button">Save Settings</button>
                </div>
            </form>
            
            <div id="message"></div>
        </div>
        <script>
            let currentSettings = {};
            
            async function loadSettings() {
                try {
                    const response = await fetch('/api/server-settings');
                    const data = await response.json();
                    currentSettings = data;
                    
                    document.getElementById('serverIP').value = data.ip || '';
                    document.getElementById('username').value = data.username || '';
                    document.getElementById('apiKey').value = data.api_key || '';
                    
                    if (data.has_password) {
                        document.getElementById('password').placeholder = 'Leave empty to keep current password';
                    }
                    
                    showMessage('Settings loaded successfully', 'success');
                } catch (error) {
                    showMessage('Error loading settings: ' + error.message, 'error');
                }
            }
            
            async function saveSettings() {
                const form = document.getElementById('kavita-form');
                const formData = new FormData(form);
                
                const settings = {
                    ip: document.getElementById('serverIP').value,
                    username: document.getElementById('username').value,
                    password: document.getElementById('password').value,
                    api_key: document.getElementById('apiKey').value
                };
                
                // Only send non-empty fields
                const filteredSettings = {};
                if (settings.ip) filteredSettings.ip = settings.ip;
                if (settings.username) filteredSettings.username = settings.username;
                if (settings.password) filteredSettings.password = settings.password;
                if (settings.api_key) filteredSettings.api_key = settings.api_key;
                
                try {
                    showMessage('Connecting to Kavita server...', 'info');
                    
                    const response = await fetch('/api/server-settings', {
                        method: 'POST',
                        headers: {
                            'Content-Type': 'application/json',
                        },
                        body: JSON.stringify(filteredSettings)
                    });
                    
                    const result = await response.json();
                    
                    if (response.ok) {
                        showMessage(result.message || 'Settings saved successfully', 'success');
                        // Clear password field
                        document.getElementById('password').value = '';
                        // Load updated settings
                        await loadSettings();
                    } else {
                        showMessage('Error: ' + (result.message || 'Failed to save settings'), 'error');
                    }
                } catch (error) {
                    showMessage('Error saving settings: ' + error.message, 'error');
                }
            }
            
            function showMessage(text, type) {
                const messageDiv = document.getElementById('message');
                messageDiv.innerHTML = '<div class="message ' + type + '">' + text + '</div>';
                setTimeout(() => {
                    messageDiv.innerHTML = '';
                }, 5000);
            }
            
            // Form submission
            document.getElementById('kavita-form').addEventListener('submit', function(e) {
                e.preventDefault();
                saveSettings();
            });
            
            // Load settings on page load
            window.addEventListener('load', loadSettings);
        </script>
    </body>
    </html>
    "#
}
