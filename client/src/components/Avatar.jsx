import md5 from "md5";
import React, { useEffect, useState } from "react";
import "./Avatar.css";

function Avatar({ name, email, size }) {
  const [avatarUrl, setAvatarUrl] = useState(null);
  const [initials, setInitials] = useState("");

  useEffect(() => {
    // Function to get initials from the name
    const getInitials = (name) => {
      if (!name) return "";
      return name
        .split(" ")
        .map((word) => word.charAt(0))
        .join("")
        .toUpperCase();
    };

    setInitials(getInitials(name));

    // Function to generate the Gravatar URL
    const generateGravatarUrl = (email) => {
      const emailHash = md5(email.trim().toLowerCase());
      const gravatarUrl = `https://www.gravatar.com/avatar/${emailHash}?s=${size}&d=404`;
      return gravatarUrl;
    };

    setAvatarUrl(generateGravatarUrl(email));
  }, [name, email, size]);

  const handleImageError = () => {
    setAvatarUrl(null);
  };

  return (
    <div
      className="avatar"
      style={{
        width: size,
        height: size,
        lineHeight: `${size}px`,
        fontSize: size / 2,
      }}
    >
      {avatarUrl ? (
        <img
          src={avatarUrl}
          alt={name}
          style={{ width: size, height: size }}
          onError={handleImageError}
        />
      ) : (
        initials
      )}
    </div>
  );
}

export default Avatar;
