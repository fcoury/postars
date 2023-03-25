import React from "react";

const Button = ({ text, onClick }) => {
  return (
    <button
      style={{
        backgroundColor: "#1E63EC",
        border: "none",
        color: "white",
        padding: "10px 16px",
        textAlign: "center",
        textDecoration: "none",
        display: "inline-block",
        fontSize: "16px",
        borderRadius: "4px",
        boxShadow: "0 4px 8px rgba(0, 0, 0, 0.2)",
        cursor: "pointer",
      }}
      onClick={onClick}
    >
      {text}
    </button>
  );
};

export default Button;
