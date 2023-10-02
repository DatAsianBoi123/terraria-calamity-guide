class Particle {
  constructor(x, y) {
    this.x = x;
    this.y = y;
    
    const xVel = Math.random() * 1.5 - 0.5;
    const yVel = Math.random() * 1.5 - 0.5;
    this.velocity = [xVel, yVel]
    this.initialVel = [xVel, yVel];
  }

  move() {
    this.x += this.velocity[0] * delta;
    this.y += this.velocity[1] * delta;

    if (this.x < -5) this.x = particleCanvas.width + 5;
    else if (this.x > particleCanvas.width + 5) this.x = -5;

    if (this.y < -5) this.y = particleCanvas.height + 5;
    else if (this.y > particleCanvas.height + 5) this.y = -5;
  }

  moveAwayFrom(x, y) {
    let dir = [this.x - x, this.y - y];
    let mag = Math.sqrt(dir[0] * dir[0] + dir[1] * dir[1]);

    if (mag == 0) return;

    let newMag = Math.pow(2, mag / -20 - 1);

    if (mag > 500) {
      this.velocity[0] -= (this.velocity[0] - this.initialVel[0]) * 0.01;
      this.velocity[1] -= (this.velocity[1] - this.initialVel[1]) * 0.01;

      return;
    }

    dir[0] *= newMag / mag;
    dir[1] *= newMag / mag;

    this.velocity[0] += dir[0];
    this.velocity[1] += dir[1];
  }

  draw() {
    particleCtx.beginPath();
    particleCtx.ellipse(this.x, this.y, 3, 3, 0, 0, Math.PI * 2);
    particleCtx.fill();
  }
}

const particleCanvas = document.getElementById('particle-bg');
resetBounds();
/** @type {CanvasRenderingContext2D} */
const particleCtx = particleCanvas.getContext('2d');

const MAX_PARTICLES = 100;
const FPS = 60;

/** @type {Particle[]} */
let particles = [];
let lastTime = Date.now();
let delta = 1;
let mouseX = 0;
let mouseY = 0;

init();
setInterval(tick, 1000 / FPS);

function resetBounds() {
  particleCanvas.width = document.body.clientWidth;
  particleCanvas.height = document.body.clientHeight;
}

function init() {
  particleCtx.fillStyle = 'white';

  while (particles.length < MAX_PARTICLES) {
    const randX = Math.floor(Math.random() * particleCanvas.width);
    const randY = Math.floor(Math.random() * particleCanvas.height);

    let particle = new Particle(randX, randY);
    particles.push(particle);
  }
}

function tick() {
  delta = (Date.now() - lastTime) / (1000 / FPS);
  lastTime = Date.now();

  particleCtx.clearRect(0, 0, particleCanvas.width, particleCanvas.height);

  for (const particle of particles) {
    particle.moveAwayFrom(mouseX, mouseY);
    particle.move();
    particle.draw();
  }
}

window.addEventListener('mousemove', e => {
  const rect = particleCanvas.getBoundingClientRect();
  mouseX = e.x + document.body.scrollLeft - rect.x;
  mouseY = e.y + document.body.scrollTop - rect.y;
});

window.addEventListener('resize', () => {
  resetBounds();
  init();
});


