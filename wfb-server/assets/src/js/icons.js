// Helper to get icon HTML string with classes
function getIcon(name, classes = "") {
    if (!name) return "";
    const url = `/images/icons/${name}.svg`;
    // Use verbose mask properties for better browser compatibility
    // and explicitly set background-color to currentColor
    return `<span class="inline-block ${classes}" style="background-color: currentColor; -webkit-mask-image: url('${url}'); mask-image: url('${url}'); -webkit-mask-repeat: no-repeat; mask-repeat: no-repeat; -webkit-mask-position: center; mask-position: center; -webkit-mask-size: contain; mask-size: contain;" aria-hidden="true"></span>`;
}
