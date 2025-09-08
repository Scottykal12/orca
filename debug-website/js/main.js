const navToggle = document.querySelector('.nav-toggle');
const nav = document.querySelector('.navbar');

navToggle.addEventListener('click', () => {
    nav.classList.toggle('nav-open');
});