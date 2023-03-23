import React from "react";
import { useAppState } from "../../state/AppState";

function EmailBody() {
  const { state } = useAppState();
  const { email } = state;

  if (!email) {
    return <div>Please select an email to view its content.</div>;
  }

  return (
    <div className="body">
      <div dangerouslySetInnerHTML={{ __html: email.body.content }} />
    </div>
  );
}

export default EmailBody;
