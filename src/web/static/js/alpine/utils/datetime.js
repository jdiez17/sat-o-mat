const pad2 = (value) => value.toString().padStart(2, '0');

export const formatDate = (value) => {
    const date = new Date(value);
    if (isNaN(date.getTime())) return value;
    return `${pad2(date.getUTCDate())}/${pad2(date.getUTCMonth() + 1)}/${date.getUTCFullYear()}`;
};

export const formatTime = (value) => {
    const date = new Date(value);
    if (isNaN(date.getTime())) return value;
    return `${pad2(date.getUTCHours())}:${pad2(date.getUTCMinutes())}:${pad2(date.getUTCSeconds())}`;
};

export const formatDateTime = (value) => {
    const date = new Date(value);
    if (isNaN(date.getTime())) return value;
    return `${formatDate(date)} ${formatTime(date)}`;
};
