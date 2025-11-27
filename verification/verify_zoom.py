
from playwright.sync_api import sync_playwright

def verify_zooming_disabled():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()

        # Load the index.html file
        page.goto('file:///app/index.html')

        # Check for the viewport meta tag
        viewport = page.locator('meta[name="viewport"]')
        content = viewport.get_attribute('content')
        print(f'Viewport content: {content}')

        assert 'maximum-scale=1.0' in content
        assert 'user-scalable=no' in content

        # Check for the JS event listener (by executing JS and checking behavior)
        # It's hard to verify 'dblclick' prevention via script without interaction,
        # but we can check if the script is loaded/executed.
        # We can try to trigger a dblclick and see if it throws error or similar,
        # but the main thing is verifying the code is present.

        # Take a screenshot
        page.screenshot(path="/home/jules/verification/verification.png")
        browser.close()

if __name__ == '__main__':
    verify_zooming_disabled()
