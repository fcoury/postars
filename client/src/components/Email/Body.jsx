import React, { useEffect, useRef } from "react";
import { useAppState } from "../../state/AppState";

function EmailBody() {
  const { state } = useAppState();
  const { email } = state;
  const iframeRef = useRef(null);

  useEffect(() => {
    if (email && iframeRef.current) {
      const iframeDoc = iframeRef.current.contentWindow.document;
      iframeDoc.open();
      iframeDoc.write(email.body.content);
      iframeDoc.close();
    }
  }, [email]);

  if (!email) {
    return <div>Please select an email to view its content.</div>;
  }

  return (
    <div className="body">
      <iframe
        ref={iframeRef}
        title="email-content"
        frameBorder="0"
        sandbox="allow-same-origin"
        style={{ flexGrow: 1, width: "100%" }}
      />
    </div>
  );
}

export default EmailBody;
