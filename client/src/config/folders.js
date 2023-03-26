const FOLDERS = [
  { icon: "far fa-inbox", folder: "Inbox", label: "Inbox" },
  { icon: "far fa-paper-plane", folder: "Sent Items", label: "Sent" },
  { icon: "far fa-exclamation-square", folder: "Junk Email", label: "Spam" },
  { icon: "far fa-mail-bulk", folder: "All Mail", label: "All Mail" },
  { icon: "far fa-trash-alt", folder: "Deleted Items", label: "Trash" },
  { icon: "far fa-search", folder: "Search", label: "Search", hidden: true },
];

export const getLabel = (folder) => {
  const folderObj = FOLDERS.find((f) => f.folder === folder);
  return folderObj.label;
};

export default FOLDERS;
